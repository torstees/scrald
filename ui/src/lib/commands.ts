// Typed wrappers around the Tauri commands, so components never call
// `invoke` with raw strings.

import { invoke } from "@tauri-apps/api/core";
import type { LaunchInfo } from "./types";

export function launchInfo(): Promise<LaunchInfo> {
  return invoke<LaunchInfo>("launch_info");
}
