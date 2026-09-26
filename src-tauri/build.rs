fn main() {
    // generate_context! embeds the icons (the Dock icon under `tauri dev`)
    // at compile time, but cargo doesn't know they're inputs, so without
    // this an icon change never reaches a dev build.
    println!("cargo:rerun-if-changed=icons");
    tauri_build::build()
}
