use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap();

    let timer_frequency_hz = env::var("TIMER_FREQUENCY_HZ").unwrap_or_else(|_| "1000".to_string());

    let out_dir = PathBuf::from("src");
    fs::write(
        out_dir.join("config.rs"),
        format!(r#"pub const TIMER_FREQUENCY_HZ: usize = {timer_frequency_hz};"#),
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
