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
use std::path::Path;

pub mod parse;
use parse::Generator;

/// Parse all `.ftl` files under `locales_dir`, validate against `primary` locale,
/// and generate a type-safe i18n Rust module to `output_path`.
pub fn generate(locales_dir: impl AsRef<Path>, output_path: impl AsRef<Path>, primary: &str) {
    let gen = Generator::load(locales_dir.as_ref(), primary);
    let code = gen.generate();
    let output_path = output_path.as_ref();
    fs::write(output_path, code)
        .unwrap_or_else(|e| panic!("Failed to write {}: {}", output_path.display(), e));
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
        generate(&locales, &out, "en-US");

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
        generate(&dir, dir.join("nope/out.rs"), "en-US");
    }
}
