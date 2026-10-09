// Typed wrappers around the Tauri commands, so components never call
// `invoke` with raw strings.

import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { ScrollAnchor } from "../reader/anchor";
import type { LaunchInfo, LinkTarget, OpenedDocument, RecentDocument } from "./types";

export function launchInfo(): Promise<LaunchInfo> {
  return invoke<LaunchInfo>("launch_info");
}

/**
 * Reads and parses a document, with what Scrald remembers about it (reading
 * position, remote images choice). Rejects with an error message string.
 */
export function openDocument(path: string): Promise<OpenedDocument> {
  return invoke<OpenedDocument>("open_document", { path });
}

/** Remembers the reading position in a document. Never throws. */
export function saveReadingPosition(path: string, anchor: ScrollAnchor): void {
  invoke("save_reading_position", { path, anchor }).catch((e: unknown) => {
    console.warn("could not save reading position", e);
  });
}

/** Remembers whether remote images may load for a document. */
export function setRemoteImages(path: string, allowed: boolean): Promise<void> {
  return invoke("set_remote_images", { path, allowed });
}

/** Recently opened documents, newest (and pinned) first. */
export function recentDocuments(limit: number): Promise<RecentDocument[]> {
  return invoke<RecentDocument[]>("recent_documents", { limit });
}

/**
 * URL of a document image served by the `scrald-asset` protocol. Tauri
 * formats custom-scheme URLs differently per platform (DESIGN.md §11.1), so
 * this is the one place they are built.
 */
export function assetUrl(assetToken: number, imageId: number): string {
  return convertFileSrc(`${assetToken}-${imageId}`, "scrald-asset");
}

/** Classifies a link clicked in the document at `documentPath`. */
export function resolveLink(documentPath: string, href: string): Promise<LinkTarget> {
  return invoke<LinkTarget>("resolve_link", { document: documentPath, href });
}

/** Opens an http(s) or mailto link in the system's default handler. */
export function openExternal(url: string): Promise<void> {
  return invoke("open_external", { url });
}

/** Calls `handler` with the path whenever this window's document changes on disk. */
export function onDocumentChanged(handler: (path: string) => void): Promise<UnlistenFn> {
  return listen<string>("document-changed", (event) => handler(event.payload));
}

/** Sends a frontend timing to the backend log. Never throws. */
export function reportTiming(name: string, ms: number): void {
  invoke("report_timing", { name, ms }).catch(() => {
    // Timing is best-effort diagnostics; losing one measurement is fine.
  });
}
