use cc;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    println!("cargo:rerun-if-changed=src/kernel_entry.S");
    cc::Build::new()
        .file("src/kernel_entry.S")
        .compile("entry");

    println!("cargo:rerun-if-changed=kernel.ld");
    println!("cargo:rustc-link-arg=-Tkernel.ld");
}
