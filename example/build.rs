use std::path::Path;

fn main() {
    let locales_dir = Path::new("locales");
    let out_dir = std::env::var("OUT_DIR").unwrap();

    println!("cargo:rerun-if-changed={}", locales_dir.display());

    ftl_codegen::generate(
        locales_dir,
        Path::new(&out_dir).join("i18n_gen.rs"),
        "en-US",
    );
}
