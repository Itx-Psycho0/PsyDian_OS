use std::env;
use std::path::PathBuf;

fn main() {
    let kernel = PathBuf::from(
        env::var_os("CARGO_BIN_FILE_KERNEL_kernel").expect("kernel binary artifact not found"),
    );

    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR not set"));
    let image = out_dir.join("psydian-bios.img");

    // Keep the bootloader's own log lines off the serial port: that line is the
    // kernel's diagnostic channel and must only carry Psydian output.
    let mut boot_config = bootloader::BootConfig::default();
    boot_config.serial_logging = false;
    boot_config.frame_buffer_logging = false;

    bootloader::BiosBoot::new(&kernel)
        .set_boot_config(&boot_config)
        .create_disk_image(&image)
        .expect("failed to create BIOS disk image");

    // Lets `src/main.rs` find the image with `env!("PSYDIAN_BIOS_IMAGE")`.
    println!("cargo:rustc-env=PSYDIAN_BIOS_IMAGE={}", image.display());
    println!("cargo:rerun-if-changed=kernel/src");
    println!("cargo:rerun-if-changed=kernel/Cargo.toml");
    println!("cargo:rerun-if-changed=build.rs");
}
