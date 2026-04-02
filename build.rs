fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    println!("cargo:rerun-if-changed=src/kernel.ld");
    println!("cargo:rustc-link-arg=-Tsrc/kernel.ld");
}
