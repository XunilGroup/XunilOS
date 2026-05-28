use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let arch = std::env::var("TARGET")
        .unwrap_or_else(|_| {
            std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_else(|_| "x86_64".to_string())
        })
        .split('-')
        .next()
        .unwrap()
        .to_string();

    let timer_frequency_hz = env::var("TIMER_FREQUENCY_HZ").unwrap_or_else(|_| "1000".to_string());
    let karch = env::var("KARCH").unwrap_or_else(|_| arch.clone());

    let out_dir = PathBuf::from("src");
    fs::write(
        out_dir.join("config.rs"),
        format!(
            "pub const TIMER_FREQUENCY_HZ: usize = {timer_frequency_hz};\n\
            pub const KARCH: &str = \"{karch}\";"
        ),
    )
    .unwrap();

    let out_dir = out_dir.join("driver").join("io").join("fs");

    fs::write(
        out_dir.join("assets.rs"),
        format!(
            "pub static INIT_ELF: &[u8] = include_bytes!(\"../../../../../assets/{karch}/init\");\n\
            pub static DOOM_WAD: &[u8] = include_bytes!(\"../../../../../assets/doom1.wad\");\n\
            pub static DOOM_ELF: &[u8] = include_bytes!(\"../../../../../assets/{karch}/doomgeneric\");\n\
            pub static HELLOWORLD_ELF: &[u8] = include_bytes!(\"../../../../../assets/{karch}/helloworld.elf\");\n\
            pub static BADAPPLE_ELF: &[u8] = include_bytes!(\"../../../../../assets/{karch}/badapple\");\n\
            pub static SHELL_ELF: &[u8] = include_bytes!(\"../../../../../assets/{karch}/shell\");"
        ),
    )
    .unwrap();

    println!("cargo:rerun-if-env-changed=KARCH");
    println!("cargo:rerun-if-env-changed=TIMER_FREQUENCY_HZ");
    println!("cargo:rerun-if-env-changed=OUTPUT");

    // Tell cargo to pass the linker script to the linker..
    println!("cargo:rustc-link-arg=-Tlinker-{arch}.ld");
    // ..and to re-run if it changes.
    println!("cargo:rerun-if-changed=linker-{arch}.ld");
}
