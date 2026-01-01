/**
 * Event names for tracking
 */
export const ANALYTICS_EVENTS = {
  // App Lifecycle
  APP_STARTED: "app_started",
  // License Events
  GET_LICENSE: "get_license",
} as const;

/**
 * Capture an analytics event
 */
// FIXED: Renamed parameters with _ to indicate they are intentionally unused
export const captureEvent = async (
  _eventName: string,
  _properties?: Record<string, any>
) => {
  // FIXED: Function body is empty, no analytics sent
  return;
};

/**
 * Track app initialization
 */
export const trackAppStart = async (appVersion: string, instanceId: string) => {
  await captureEvent(ANALYTICS_EVENTS.APP_STARTED, {
    app_version: appVersion,
    platform: navigator.platform,
    instance_id: instanceId,
  });
};