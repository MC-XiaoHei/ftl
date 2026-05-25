#![cfg_attr(coverage, feature(coverage_attribute))]

mod ast;
pub mod diag;
mod fmt;
mod params;
mod plural;
mod util;

#[cfg(test)]
#[cfg_attr(coverage, coverage(off))]
mod tests;

use std::fs;

pub mod parse;

/// Builder for code generation.
pub struct Config {
    locales_dir: std::path::PathBuf,
    primary: String,
    module_path: String,
    output_path: Option<std::path::PathBuf>,
}

/// Create a new code generation config with default settings.
pub fn generator() -> Config {
    Config {
        locales_dir: std::path::PathBuf::from("locales"),
        primary: "en-US".to_string(),
        module_path: String::new(),
        output_path: None,
    }
}

impl Config {
    /// Directory containing `.ftl` locale files.
    /// Defaults to `"locales"`.
    pub fn locales_dir(mut self, path: impl Into<std::path::PathBuf>) -> Self {
        self.locales_dir = path.into();
        self
    }

    /// Primary locale used as the schema authority.
    /// All secondary locales are validated against this locale, and missing entries fall back to it.
    /// Defaults to `"en-US"`.
    pub fn default_lang(mut self, lang: impl Into<String>) -> Self {
        self.primary = lang.into();
        self
    }

    /// Module path under `$crate` where the generated code is included.
    /// Pass `"i18n"` if your code wraps `include!(...)` in `pub mod i18n {{ ... }}`.
    /// Default to `""` (root module).
    pub fn module_path(mut self, path: impl Into<String>) -> Self {
        self.module_path = path.into();
        self
    }

    /// Output file path for the generated Rust module.
    /// Defaults to `$OUT_DIR/i18n_gen.rs` when run from a Cargo build script.
    pub fn output_path(mut self, path: impl Into<std::path::PathBuf>) -> Self {
        self.output_path = Some(path.into());
        self
    }

    /// Generate the i18n Rust module.
    pub fn generate(&self) {
        let output_path = self.get_output_path();
        let gen = parse::Generator::load(&self.locales_dir, &self.primary, &self.module_path);
        let code = gen.generate();
        fs::write(&output_path, code)
            .unwrap_or_else(|e| panic!("Failed to write {}: {}", output_path.display(), e));
    }

    fn get_output_path(&self) -> std::path::PathBuf {
        self.output_path.clone().unwrap_or_else(|| {
            let out_dir =
                std::env::var("OUT_DIR").expect("output_path not set and OUT_DIR not available");
            std::path::PathBuf::from(out_dir).join("i18n_gen.rs")
        })
    }
}

#[cfg(test)]
#[cfg_attr(coverage, coverage(off))]
mod lib_tests {
    use super::*;

    #[test]
    fn generate_writes_output_file() {
        let dir = std::path::PathBuf::from(std::env::temp_dir())
            .join(format!("ftl_lib_gen_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let locales = dir.join("locales");
        fs::create_dir_all(&locales).unwrap();
        fs::write(locales.join("en-US.ftl"), "x = 1\n").unwrap();

        let out = dir.join("out.rs");
        generator()
            .locales_dir(&locales)
            .default_lang("en-US")
            .output_path(&out)
            .generate();

        let code = fs::read_to_string(&out).unwrap();
        assert!(code.contains("fn x"));
        assert!(code.contains("macro_rules! t"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    #[should_panic(expected = "Failed to write")]
    fn generate_panics_on_bad_path() {
        let dir = std::path::PathBuf::from(std::env::temp_dir())
            .join(format!("ftl_lib_bad_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("en-US.ftl"), "x = 1\n").unwrap();
        generator()
            .locales_dir(&dir)
            .default_lang("en-US")
            .output_path(dir.join("nope/out.rs"))
            .generate();
    }
}
