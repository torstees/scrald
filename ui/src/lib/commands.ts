// Typed wrappers around the Tauri commands, so components never call
// `invoke` with raw strings.

import { invoke } from "@tauri-apps/api/core";
import type { DocumentModel, LaunchInfo } from "./types";

export function launchInfo(): Promise<LaunchInfo> {
  return invoke<LaunchInfo>("launch_info");
}

/** Reads and parses a document; rejects with an error message string. */
export function openDocument(path: string): Promise<DocumentModel> {
  return invoke<DocumentModel>("open_document", { path });
}

/** Sends a frontend timing to the backend log. Never throws. */
export function reportTiming(name: string, ms: number): void {
  invoke("report_timing", { name, ms }).catch(() => {
    // Timing is best-effort diagnostics; losing one measurement is fine.
  });
}
