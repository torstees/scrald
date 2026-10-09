// TypeScript mirrors of the Tauri command payloads in
// crates/scrald-app/src/commands.rs. Keep these in sync with the Rust types.

/** Mirrors `LaunchInfo` (serde `rename_all = "camelCase"`). */
export interface LaunchInfo {
  /** Absolute path from the command line, or null if none was given. */
  path: string | null;
  /** Display title for the window and page. */
  title: string;
  /** Whether `path` existed when the app started. */
  exists: boolean;
}
