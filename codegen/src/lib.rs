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
