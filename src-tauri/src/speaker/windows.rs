// Pluely windows speaker input and stream
use anyhow::Result;
use futures_util::Stream;
use std::collections::VecDeque;
use std::sync::{mpsc, Arc, Mutex};
use std::task::{Poll, Waker};
use std::thread;
use std::time::Duration;
use tracing::error;
// FIX: Updated imports for wasapi 0.22.0
use wasapi::{
    AudioCaptureClient, DeviceEnumerator, Direction, Handle, SampleType, WaveFormat, StreamMode,
};

pub struct SpeakerInput {
    device_index: Option<usize>,
}

impl SpeakerInput {
    pub fn new(device_id: Option<String>) -> Result<Self> {
        let device_index = device_id.and_then(|id| {
            id.strip_prefix("windows_output_")
                .and_then(|s| s.parse::<usize>().ok())
        });

        Ok(Self { device_index })
    }

    pub fn stream(self) -> SpeakerStream {
        let sample_queue = Arc::new(Mutex::new(VecDeque::new()));
        let waker_state = Arc::new(Mutex::new(WakerState {
            waker: None,
            has_data: false,
            shutdown: false,
        }));
        let (init_tx, init_rx) = mpsc::channel();

        let queue_clone = sample_queue.clone();
        let waker_clone = waker_state.clone();
        let device_index = self.device_index;

        let capture_thread = thread::spawn(move || {
            if let Err(e) =
                SpeakerStream::capture_audio_loop(queue_clone, waker_clone, init_tx, device_index)
            {
                error!("Pluely Audio capture loop failed: {}", e);
            }
        });

        let actual_sample_rate = match init_rx.recv_timeout(Duration::from_secs(5)) {
            Ok(Ok(rate)) => rate,
            Ok(Err(e)) => {
                error!("Pluely Audio initialization failed: {}", e);
                44100
            }
            Err(_) => {
                error!("Pluely Audio initialization timeout");
                44100
            }
        };

        SpeakerStream {
            sample_queue,
            waker_state,
            capture_thread: Some(capture_thread),
            actual_sample_rate,
        }
    }
}

struct WakerState {
    waker: Option<Waker>,
    has_data: bool,
    shutdown: bool,
}

pub struct SpeakerStream {
    sample_queue: Arc<Mutex<VecDeque<f32>>>,
    waker_state: Arc<Mutex<WakerState>>,
    capture_thread: Option<thread::JoinHandle<()>>,
    actual_sample_rate: u32,
}

impl SpeakerStream {
    pub fn sample_rate(&self) -> u32 {
        self.actual_sample_rate
    }

    fn capture_audio_loop(
        sample_queue: Arc<Mutex<VecDeque<f32>>>,
        waker_state: Arc<Mutex<WakerState>>,
        init_tx: mpsc::Sender<Result<u32>>,
        device_index: Option<usize>,
    ) -> Result<()> {
        let init_result: Result<(Handle, AudioCaptureClient, u32), anyhow::Error> = (|| {
            // FIX: Use DeviceEnumerator for 0.22.0
            let enumerator = DeviceEnumerator::new()?;
            
            let device = match device_index {
                Some(index) => {
                    // FIX: get_device_collection takes 1 arg (reference to direction)
                    let collection = enumerator.get_device_collection(&Direction::Render)?;
                    collection.get_device_at_index(index.try_into()?)?
                }
                // FIX: get_default_device is the correct method name on enumerator
                None => enumerator.get_default_device(&Direction::Render)?,
            };

            let mut audio_client = device.get_iaudioclient()?;

            let device_format = audio_client.get_mixformat()?;
            let actual_rate = device_format.get_samplespersec();

            // Setup format
            let desired_format =
                WaveFormat::new(32, 32, &SampleType::Float, actual_rate as usize, 1, None);

            // FIX: Restore StreamMode::EventsShared which worked in user's original code
            // and fits the 3-argument initialize_client signature.
            // Loopback is implicit when using Direction::Capture on a Render device.
            let mode = StreamMode::EventsShared { 
                autoconvert: true, 
                buffer_duration_hns: 0 // Use default or specific duration if needed
            };

            audio_client.initialize_client(
                &desired_format,
                &Direction::Capture,
                &mode,
            ).map_err(|e| {
                error!("Failed to initialize audio client: {}", e);
                e
            })?;

            let h_event = audio_client.set_get_eventhandle()?;
            let render_client = audio_client.get_audiocaptureclient()?;

            audio_client.start_stream()?;

            Ok((h_event, render_client, actual_rate))
        })();

        match init_result {
            Ok((h_event, render_client, sample_rate)) => {
                let _ = init_tx.send(Ok(sample_rate));

                loop {
                    {
                        let state = waker_state.lock().unwrap();
                        if state.shutdown {
                            break;
                        }
                    }

                    // FIX: Explicitly ignore result to satisfy type inference if needed
                    if h_event.wait_for_event(3000).is_err() {
                        error!("Pluely timeout error, stopping capture");
                        break;
                    }

                    let mut temp_queue = VecDeque::new();
                    
                    // FIX: Handle the Result<BufferInfo, ...> return type correctly
                    // We ignore BufferInfo for now as we just want the data in temp_queue
                    if let Err(e) = render_client.read_from_device_to_deque(&mut temp_queue) {
                        error!("Pluely Failed to read audio data: {}", e);
                        continue;
                    }

                    if temp_queue.is_empty() {
                        continue;
                    }

                    let mut samples = Vec::new();
                    while temp_queue.len() >= 4 {
                        let bytes = [
                            temp_queue.pop_front().unwrap(),
                            temp_queue.pop_front().unwrap(),
                            temp_queue.pop_front().unwrap(),
                            temp_queue.pop_front().unwrap(),
                        ];
                        let sample = f32::from_le_bytes(bytes);
                        samples.push(sample);
                    }

                    if !samples.is_empty() {
                        let dropped = {
                            let mut queue = sample_queue.lock().unwrap();
                            let max_buffer_size = 131072;

                            queue.extend(samples.iter());

                            if queue.len() > max_buffer_size {
                                let to_drop = queue.len() - max_buffer_size;
                                queue.drain(0..to_drop);
                                to_drop
                            } else {
                                0
                            }
                        };

                        if dropped > 0 {
                            error!("Windows buffer overflow - dropped {} samples", dropped);
                        }

                        {
                            let mut state = waker_state.lock().unwrap();
                            if !state.has_data {
                                state.has_data = true;
                                if let Some(waker) = state.waker.take() {
                                    drop(state);
                                    waker.wake();
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                let _ = init_tx.send(Err(e));
                return Ok(());
            }
        }

        Ok(())
    }
}

impl Drop for SpeakerStream {
    fn drop(&mut self) {
        {
            let mut state = self.waker_state.lock().unwrap();
            state.shutdown = true;
        }

        if let Some(thread) = self.capture_thread.take() {
            if let Err(e) = thread.join() {
                error!("Failed to join capture thread: {:?}", e);
            }
        }
    }
}

impl Stream for SpeakerStream {
    type Item = f32;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        {
            let state = self.waker_state.lock().unwrap();
            if state.shutdown {
                return Poll::Ready(None);
            }
        }

        {
            let mut queue = self.sample_queue.lock().unwrap();
            if let Some(sample) = queue.pop_front() {
                return Poll::Ready(Some(sample));
            }
        }

        {
            let mut state = self.waker_state.lock().unwrap();
            if state.shutdown {
                return Poll::Ready(None);
            }
            state.has_data = false;
            state.waker = Some(cx.waker().clone());
            drop(state);
        }

        {
            let mut queue = self.sample_queue.lock().unwrap();
            match queue.pop_front() {
                Some(sample) => Poll::Ready(Some(sample)),
                None => Poll::Pending,
            }
        }
    }
}