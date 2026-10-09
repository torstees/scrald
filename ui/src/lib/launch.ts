import type { LaunchInfo } from "./types";

/** One-line status describing what the app was launched with. */
export function launchMessage(info: LaunchInfo): string {
  if (info.path === null) {
    return "No document open. Pass a Markdown file on the command line.";
  }
  if (!info.exists) {
    return `File not found: ${info.path}`;
  }
  return `Opened ${info.path}`;
}
