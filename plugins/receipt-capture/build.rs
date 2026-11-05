fn main() {
    // There are deliberately no guest-JavaScript commands. Trusted host Rust calls
    // the native API after validating app state and opaque receipt references.
    tauri_plugin::Builder::new(&[])
        .android_path("android")
        .build();
}
