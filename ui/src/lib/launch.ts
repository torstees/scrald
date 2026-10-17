import type { LaunchInfo } from "./types";

/**
 * One-line status describing what the app was launched with; null when it
 * was launched without a file (the start screen says what to do).
 */
export function launchMessage(info: LaunchInfo): string | null {
  if (info.path === null) {
    return null;
  }
  if (!info.exists) {
    return `File not found: ${info.path}`;
  }
  return `Opened ${info.path}`;
}
