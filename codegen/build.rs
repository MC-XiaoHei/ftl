fn main() {
    println!("cargo::rustc-check-cfg=cfg(coverage)");
    println!("cargo::rustc-check-cfg=cfg(coverage_nightly)");
}
