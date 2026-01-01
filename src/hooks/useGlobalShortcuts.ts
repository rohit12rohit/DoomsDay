import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect } from "react";
import { getShortcutsConfig } from "@/lib";

// === SINGLETON STATE ===
// This persists across all hook instances.

const registry = {
  audio: new Set<() => void>(),
  screenshot: new Set<() => void | Promise<void>>(),
  systemAudio: new Set<() => void>(),
  custom: new Map<string, Set<() => void>>(),
  inputRefs: new Set<HTMLInputElement>(),
};

let isListenersInitialized = false;

// Initialize listeners exactly once for the entire application lifecycle
const initializeGlobalListeners = async () => {
  if (isListenersInitialized) return;
  isListenersInitialized = true;

  try {
    // 1. Focus Input
    await listen("focus-text-input", () => {
      setTimeout(() => {
        // Focus the first valid input registered
        for (const input of registry.inputRefs) {
          if (input && document.body.contains(input)) {
            input.focus();
            break; // Focus only one
          }
        }
      }, 100);
    });

    // 2. Audio Recording
    await listen("start-audio-recording", () => {
      registry.audio.forEach((cb) => cb());
    });

    // 3. Screenshot (Debounced)
    let lastScreenshot = 0;
    await listen("trigger-screenshot", () => {
      const now = Date.now();
      if (now - lastScreenshot < 300) return;
      lastScreenshot = now;
      
      registry.screenshot.forEach((cb) => {
        try { Promise.resolve(cb()); } catch (e) { console.error(e); }
      });
    });

    // 4. System Audio
    await listen("toggle-system-audio", () => {
      registry.systemAudio.forEach((cb) => cb());
    });

    // 5. Custom Shortcuts
    await listen<{ action: string }>("custom-shortcut-triggered", (event) => {
      const callbacks = registry.custom.get(event.payload.action);
      if (callbacks) {
        callbacks.forEach((cb) => cb());
      }
    });

    // 6. Errors
    await listen<Array<[string, string, string]>>("shortcut-registration-error", (event) => {
       window.dispatchEvent(
        new CustomEvent("shortcutRegistrationError", { detail: event.payload })
      );
    });

    console.log("Global shortcut listeners initialized successfully.");
  } catch (error) {
    console.error("Failed to initialize global shortcut listeners:", error);
    isListenersInitialized = false; // Retry next time if failed
  }
};

export const useGlobalShortcuts = () => {
  // Ensure listeners are set up when the hook is first used
  useEffect(() => {
    initializeGlobalListeners();
  }, []);

  const checkShortcutsRegistered = useCallback(async (): Promise<boolean> => {
    try {
      return await invoke<boolean>("check_shortcuts_registered");
    } catch (error) {
      console.error("Failed to check shortcuts:", error);
      return false;
    }
  }, []);

  const getShortcuts = useCallback(async (): Promise<Record<string, string> | null> => {
    try {
      return await invoke<Record<string, string>>("get_registered_shortcuts");
    } catch (error) {
      console.error("Failed to get shortcuts:", error);
      return null;
    }
  }, []);

  const updateShortcuts = useCallback(async (): Promise<boolean> => {
    try {
      const config = getShortcutsConfig();
      await invoke("update_shortcuts", { config });
      return true;
    } catch (error) {
      console.error("Failed to update shortcuts:", error);
      return false;
    }
  }, []);

  // --- Registration Functions (Add to Registry) ---

  const registerInputRef = useCallback((input: HTMLInputElement | null) => {
    if (input) {
      registry.inputRefs.add(input);
    }
    // Return cleanup function
    return () => {
      if (input) registry.inputRefs.delete(input);
    };
  }, []);

  const registerAudioCallback = useCallback((callback: () => void) => {
    registry.audio.add(callback);
    // Auto-cleanup when the component unmounts or callback changes
    return () => registry.audio.delete(callback);
  }, []);

  const registerScreenshotCallback = useCallback((callback: () => void | Promise<void>) => {
    registry.screenshot.add(callback);
    return () => registry.screenshot.delete(callback);
  }, []);

  const registerSystemAudioCallback = useCallback((callback: () => void) => {
    registry.systemAudio.add(callback);
    return () => registry.systemAudio.delete(callback);
  }, []);

  const registerCustomShortcutCallback = useCallback((actionId: string, callback: () => void) => {
    if (!registry.custom.has(actionId)) {
      registry.custom.set(actionId, new Set());
    }
    registry.custom.get(actionId)?.add(callback);
  }, []);

  const unregisterCustomShortcutCallback = useCallback((actionId: string) => {
    // Note: This removes ALL callbacks for an action. 
    registry.custom.delete(actionId);
  }, []);

  return {
    checkShortcutsRegistered,
    getShortcuts,
    updateShortcuts,
    registerInputRef,
    registerAudioCallback,
    registerScreenshotCallback,
    registerSystemAudioCallback,
    registerCustomShortcutCallback,
    unregisterCustomShortcutCallback,
  };
};