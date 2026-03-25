use std::path::PathBuf;

fn main() {
    // Resolve absolute path to the TUI root (two levels up from tauri/src-tauri)
    // CARGO_MANIFEST_DIR = .../theta-vault-rust/tauri/src-tauri
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let tui_dir = PathBuf::from(&manifest_dir)
        .join("../..")
        .canonicalize()
        .expect("theta-vault-rust root not found relative to tauri/src-tauri");

    // Export compile-time env vars consumed by env!() in main.rs
    println!("cargo:rustc-env=TUI_CWD={}", tui_dir.display());
    println!("cargo:rustc-env=TUI_BINARY_BASE={}", tui_dir.join("target").display());

    // Re-run this build script when TUI source or manifest changes
    println!("cargo:rerun-if-changed={}/src", tui_dir.display());
    println!("cargo:rerun-if-changed={}/Cargo.toml", tui_dir.display());

    // Auto-build the TUI binary so the user only needs `npm run tauri dev`
    let profile = if cfg!(debug_assertions) { "dev" } else { "release" };
    let mut args = vec![
        "build".to_string(),
        "--manifest-path".to_string(),
        tui_dir.join("Cargo.toml").to_string_lossy().into_owned(),
    ];
    if profile == "release" {
        args.push("--release".to_string());
    }

    let status = std::process::Command::new("cargo")
        .args(&args)
        .status()
        .expect("Failed to invoke cargo build for theta-vault-rust");

    if !status.success() {
        panic!("theta-vault-rust TUI build failed");
    }

    tauri_build::build()
}
