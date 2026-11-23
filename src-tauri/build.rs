fn main() {
    tauri_build::build();
    // The opt-in GUI example also needs the Windows Common Controls v6 manifest.
    // tauri-build normally links this resource only into application binaries.
    if cfg!(windows) && std::env::var_os("CARGO_FEATURE_BROWSER_SMOKE").is_some() {
        let resource =
            std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("resource.lib");
        println!("cargo:rustc-link-arg-examples={}", resource.display());
    }
}
