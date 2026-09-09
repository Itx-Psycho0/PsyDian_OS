use std::env;
use std::path::PathBuf;

fn main() {
    let kernel = PathBuf::from(
        env::var_os("CARGO_BIN_FILE_KERNEL_kernel").expect("kernel binary artifact not found"),
    );

    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR not set"));

    bootloader::BiosBoot::new(&kernel)
        .create_disk_image(&out_dir.join("psydian-bios.img"))
        .expect("failed to create BIOS disk image");

    println!("cargo:rerun-if-changed=kernel/src");
}
