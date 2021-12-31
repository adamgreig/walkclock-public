use std::env;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

fn main() {
    // Copy `memory.x` to OUT_DIR.
    let out = &PathBuf::from(env::var_os("OUT_DIR").unwrap());
    File::create(out.join("memory.x"))
        .unwrap()
        .write_all(include_bytes!("memory.x"))
        .unwrap();
    println!("cargo:rustc-link-search={}", out.display());

    // Rebuild when `memory.x` changes.
    println!("cargo:rerun-if-changed=memory.x");

    // Set link script.
    println!("cargo:rustc-link-arg=-Tlink.x");
}
