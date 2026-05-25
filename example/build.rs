fn main() {
    println!("cargo:rerun-if-changed=locales");
    ftl_codegen::generator().module_path("i18n").generate();
}
