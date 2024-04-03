use std::path::Path;

fn main() {
    let librime_lib_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("librime/dist/lib");
    let librime_lib_dir = librime_lib_dir.to_str().unwrap();
    // for loading rime shared library in cargo run / cargo test.
    if cfg!(target_os = "macos") {
        println!("cargo:rustc-env=DYLD_FALLBACK_LIBRARY_PATH={librime_lib_dir}");
    } else if cfg!(unix) {
        println!("cargo:rustc-env=LD_LIBRARY_PATH={librime_lib_dir}");
    } else if cfg!(windows) {
        println!("cargo:rustc-env=PATH={librime_lib_dir}");
    }
}
