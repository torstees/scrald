fn main() {
    // tauri-build compiles icons/icon.ico into the Windows exe, but only re-runs when
    // tauri.conf.json or capabilities/ change, so a new icon set would otherwise be
    // left out of the next build. Cargo checks every file under a listed folder.
    println!("cargo:rerun-if-changed=icons");
    // Generates Tauri's build-time context (app manifest, Windows resources, icons).
    tauri_build::build()
}
