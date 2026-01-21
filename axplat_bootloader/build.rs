//! Build script for axplat_bootloader

use std::env;
use std::fs;
use std::path::Path;

fn main() {
    let arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();
    let out_dir = env::var("OUT_DIR").unwrap();

    println!("cargo:rerun-if-changed=build.rs");

    match arch.as_str() {
        "aarch64" => {
            // Copy linker script to output directory
            let linker_script = "linker-aarch64.lds";
            println!("cargo:rerun-if-changed={}", linker_script);

            let src = Path::new(linker_script);
            let dst = Path::new(&out_dir).join("linker.lds");

            fs::copy(src, &dst).expect("Failed to copy linker script");

            // Tell cargo to use this linker script
            println!("cargo:rustc-link-arg=-T{}", dst.display());

            println!("cargo:warning=Using linker script: {}", dst.display());
        }
        "x86_64" => {
            println!("cargo:warning=x86_64 linker script not yet implemented");
        }
        "riscv64" => {
            println!("cargo:warning=riscv64 linker script not yet implemented");
        }
        _ => {
            panic!("Unsupported architecture:  {}", arch);
        }
    }
}
