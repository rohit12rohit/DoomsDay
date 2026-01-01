import { useEffect } from "react";
import { useGlobalShortcuts } from "./useGlobalShortcuts";

interface UseShortcutsProps {
  onAudioRecording?: () => void;
  onScreenshot?: () => void;
  onSystemAudio?: () => void;
  customShortcuts?: Record<string, () => void>;
}

export const useShortcuts = ({
  onAudioRecording,
  onScreenshot,
  onSystemAudio,
  customShortcuts = {},
}: UseShortcutsProps = {}) => {
  const {
    registerAudioCallback,
    registerScreenshotCallback,
    registerSystemAudioCallback,
    registerCustomShortcutCallback,
    unregisterCustomShortcutCallback,
    ...rest
  } = useGlobalShortcuts();

  useEffect(() => {
    if (onAudioRecording) {
      const cleanup = registerAudioCallback(onAudioRecording);
      // Fix: Wrap in a function that returns void
      return () => { cleanup(); };
    }
  }, [onAudioRecording, registerAudioCallback]);

  useEffect(() => {
    if (onScreenshot) {
      const cleanup = registerScreenshotCallback(onScreenshot);
      return () => { cleanup(); };
    }
  }, [onScreenshot, registerScreenshotCallback]);

  useEffect(() => {
    if (onSystemAudio) {
      const cleanup = registerSystemAudioCallback(onSystemAudio);
      return () => { cleanup(); };
    }
  }, [onSystemAudio, registerSystemAudioCallback]);

  // Register custom shortcut callbacks
  useEffect(() => {
    Object.entries(customShortcuts).forEach(([actionId, callback]) => {
      registerCustomShortcutCallback(actionId, callback);
    });

    return () => {
      Object.keys(customShortcuts).forEach((actionId) => {
        unregisterCustomShortcutCallback(actionId);
      });
    };
  }, [
    customShortcuts,
    registerCustomShortcutCallback,
    unregisterCustomShortcutCallback,
  ]);

  return {
    registerAudioCallback,
    registerScreenshotCallback,
    registerSystemAudioCallback,
    registerCustomShortcutCallback,
    unregisterCustomShortcutCallback,
    ...rest,
  };
};