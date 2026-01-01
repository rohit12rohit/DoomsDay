use tauri::{AppHandle, Manager, Runtime, WebviewWindow};

// Define the fixed height and offset
pub const WINDOW_HEIGHT: f64 = 54.0;
pub const TOP_OFFSET: f64 = 0.0;

#[tauri::command]
pub fn set_window_height(window: WebviewWindow, height: f64) {
    let _ = window.set_size(tauri::Size::Logical(tauri::LogicalSize {
        width: 600.0,
        height,
    }));
}

#[tauri::command]
pub fn open_dashboard(app: AppHandle) {
    if let Some(window) = app.get_webview_window("dashboard") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

#[tauri::command]
pub fn toggle_dashboard<R: Runtime>(app: AppHandle<R>) {
    if let Some(window) = app.get_webview_window("dashboard") {
        if window.is_visible().unwrap_or(false) {
            let _ = window.hide();
        } else {
            let _ = window.show();
            let _ = window.set_focus();
        }
    }
}

#[tauri::command]
pub fn move_window(_window: WebviewWindow, direction: String) {
    println!("Move window command received: {}", direction);
}

pub fn setup_main_window<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let window = app.get_webview_window("main").unwrap();

    if let Some(monitor) = window.current_monitor()? {
        let screen_size = monitor.size();
        let scale_factor = monitor.scale_factor();
        let logical_width = 600.0; 
        
        let x = (screen_size.width as f64 / scale_factor - logical_width) / 2.0;
        let y = TOP_OFFSET;

        window.set_position(tauri::Position::Logical(tauri::LogicalPosition { x, y }))?;
    }

    #[cfg(target_os = "windows")]
    {
        use windows::Win32::Foundation::HWND;
        use windows::Win32::UI::WindowsAndMessaging::{
            GetWindowLongPtrW, SetWindowLongPtrW, GWL_EXSTYLE, WS_EX_NOACTIVATE
        };

        // FIX: Cast for windows 0.62.0 compatibility (HWND wraps *mut void now)
        let hwnd_val = window.hwnd().unwrap().0;
        let hwnd_ptr = HWND(hwnd_val as *mut std::ffi::c_void);

        unsafe {
            let current_style = GetWindowLongPtrW(hwnd_ptr, GWL_EXSTYLE);
            let new_style = current_style | (WS_EX_NOACTIVATE.0 as isize);
            SetWindowLongPtrW(hwnd_ptr, GWL_EXSTYLE, new_style);
        }
        
        window.set_always_on_top(true)?;
    }

    #[cfg(target_os = "macos")]
    {
        use tauri_nspanel::WebviewWindowExt;
        window.set_visible_on_all_workspaces(true)?;
    }

    Ok(())
}

pub fn create_dashboard_window(_app: &AppHandle) -> tauri::Result<()> {
    Ok(())
}