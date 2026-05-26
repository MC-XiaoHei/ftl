#![cfg_attr(coverage, feature(coverage_attribute))]

mod ast;
pub mod diag;
mod fmt;
mod params;
mod plural;
mod util;

use std::collections::BTreeMap;
use std::fs;

pub mod parse;

#[derive(Clone)]
pub struct BuiltInNamedArg {
    /// FTL name (camelCase, e.g. "minimumFractionDigits").
    pub ftl_name: String,
    /// Rust name (snake_case, e.g. "minimum_fraction_digits").
    pub rust_name: String,
    pub arg_type: BuiltInArgType,
}

#[derive(Clone)]
pub enum BuiltInArgType {
    String,
    Int,
    Float,
    Bool,
}

#[derive(Clone)]
pub struct BuiltInFuncDef {
    /// FTL function name (e.g. "NUMBER").
    pub name: String,
    /// Rust type name (e.g. "Number").
    pub ty_name: String,
    pub named_args: Vec<BuiltInNamedArg>,
    /// Raw `write_to` source; when set, codegen emits the type.
    pub write_to_body: Option<String>,
}

/// Produce a [`BuiltInFuncDef`] for registration.
///
/// With `impl |this, out| { ... }`, codegen emits the type definition.
/// Only `build.rs` is needed — no duplicate in `src/`.
#[macro_export]
macro_rules! ftl_builtin {
    (
        $name:ident ( $base:ty ) {
            $( $arg:ident : $ty:tt ),* $(,)?
        }
        $( impl |$this:ident, $out:ident| $body:block )?
    ) => {
        $crate::BuiltInFuncDef {
            name: stringify!($name).to_uppercase(),
            ty_name: stringify!($name).to_string(),
            named_args: vec![
                $( $crate::__builtin_named_arg!($arg, $ty) ),*
            ],
            write_to_body: $crate::__opt_stringify!($($body)?),
        }
    };
}

#[macro_export]
macro_rules! __opt_stringify {
    () => {
        None
    };
    ($body:block) => {
        Some(stringify!($body).to_string())
    };
}

#[macro_export]
macro_rules! __builtin_named_arg {
    ($arg:ident, String) => {
        $crate::BuiltInNamedArg {
            ftl_name: $crate::__snake_to_camel(stringify!($arg)),
            rust_name: stringify!($arg).to_string(),
            arg_type: $crate::BuiltInArgType::String,
        }
    };
    ($arg:ident, i64) => {
        $crate::BuiltInNamedArg {
            ftl_name: $crate::__snake_to_camel(stringify!($arg)),
            rust_name: stringify!($arg).to_string(),
            arg_type: $crate::BuiltInArgType::Int,
        }
    };
    ($arg:ident, f64) => {
        $crate::BuiltInNamedArg {
            ftl_name: $crate::__snake_to_camel(stringify!($arg)),
            rust_name: stringify!($arg).to_string(),
            arg_type: $crate::BuiltInArgType::Float,
        }
    };
    ($arg:ident, bool) => {
        $crate::BuiltInNamedArg {
            ftl_name: $crate::__snake_to_camel(stringify!($arg)),
            rust_name: stringify!($arg).to_string(),
            arg_type: $crate::BuiltInArgType::Bool,
        }
    };
}

#[doc(hidden)]
pub fn __snake_to_camel(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut upper_next = false;
    for c in s.chars() {
        if c == '_' {
            upper_next = true;
        } else if upper_next {
            out.push(c.to_ascii_uppercase());
            upper_next = false;
        } else {
            out.push(c);
        }
    }
    out
}

#[cfg(test)]
#[cfg_attr(coverage, coverage(off))]
mod tests;

/// Builder for code generation.
pub struct Config {
    locales_dir: std::path::PathBuf,
    primary: String,
    module_path: String,
    output_path: Option<std::path::PathBuf>,
    builtins: BTreeMap<String, BuiltInFuncDef>,
}

/// Create a new code generation config with default settings.
pub fn generator() -> Config {
    Config {
        locales_dir: std::path::PathBuf::from("locales"),
        primary: "en-US".to_string(),
        module_path: String::new(),
        output_path: None,
        builtins: BTreeMap::new(),
    }
}

impl Config {
    /// Directory containing `.ftl` locale files.
    pub fn locales_dir(mut self, path: impl Into<std::path::PathBuf>) -> Self {
        self.locales_dir = path.into();
        self
    }

    /// Primary locale used as the schema authority.
    pub fn default_lang(mut self, lang: impl Into<String>) -> Self {
        self.primary = lang.into();
        self
    }

    /// Module path under `$crate` where the generated code is included.
    pub fn module_path(mut self, path: impl Into<String>) -> Self {
        self.module_path = path.into();
        self
    }

    /// Output file path for the generated Rust module.
    pub fn output_path(mut self, path: impl Into<std::path::PathBuf>) -> Self {
        self.output_path = Some(path.into());
        self
    }

    /// Register a built-in function definition.
    pub fn register(mut self, def: BuiltInFuncDef) -> Self {
        self.builtins.insert(def.name.to_string(), def);
        self
    }

    /// Generate the i18n Rust module.
    pub fn generate(&self) {
        let output_path = self.get_output_path();
        let gen = parse::Generator::load(
            &self.locales_dir,
            &self.primary,
            &self.module_path,
            &self.builtins,
        );
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
    fn generate_with_builtin_registration() {
        let dir = std::path::PathBuf::from(std::env::temp_dir())
            .join(format!("ftl_lib_builtin_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let locales = dir.join("locales");
        fs::create_dir_all(&locales).unwrap();
        fs::write(
            locales.join("en-US.ftl"),
            "x = { NUMBER($n, minimumFractionDigits: 2) }\n",
        )
        .unwrap();

        let out = dir.join("out.rs");
        let number_def = ftl_builtin! {
            Number(FluentNum) {
                minimum_fraction_digits: i64,
                style: String,
            }
        };
        generator()
            .locales_dir(&locales)
            .default_lang("en-US")
            .output_path(&out)
            .register(number_def)
            .generate();

        let code = fs::read_to_string(&out).unwrap();
        assert!(
            code.contains("minimum_fraction_digits(2i64)"),
            "code should reference the named arg builder"
        );
        assert!(code.contains("write_to"), "code should call write_to");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn builtin_registration_generates_valid_rust() {
        let dir = std::path::PathBuf::from(std::env::temp_dir())
            .join(format!("ftl_lib_builtin_emit_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let locales = dir.join("locales");
        fs::create_dir_all(&locales).unwrap();
        fs::write(
            locales.join("en-US.ftl"),
            "price = Price: { NUMBER($amount, minimumFractionDigits: 2, style: \"decimal\") }\n",
        )
        .unwrap();

        let out = dir.join("out.rs");
        let number_def = ftl_builtin! {
            Number(FluentNum) {
                minimum_fraction_digits: i64,
                style: String,
            }
        };
        generator()
            .locales_dir(&locales)
            .default_lang("en-US")
            .output_path(&out)
            .register(number_def)
            .generate();

        let code = fs::read_to_string(&out).unwrap();
        assert!(code.contains("amount: Number"), "should use Number type");
        assert!(
            code.contains("minimum_fraction_digits(2i64)"),
            "should have min frac digits"
        );
        assert!(
            code.contains("style(\"decimal\".to_string())"),
            "should have style arg"
        );
        assert!(code.contains(".write_to(&mut s)"), "should call write_to");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn snake_to_camel_conversions() {
        assert_eq!(
            super::__snake_to_camel("minimum_fraction_digits"),
            "minimumFractionDigits"
        );
        assert_eq!(super::__snake_to_camel("style"), "style");
        assert_eq!(super::__snake_to_camel("use_grouping"), "useGrouping");
        assert_eq!(super::__snake_to_camel("compact_display"), "compactDisplay");
    }

    #[test]
    fn builtin_with_body_emits_type_definition() {
        let dir = std::path::PathBuf::from(std::env::temp_dir())
            .join(format!("ftl_lib_builtin_body_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let locales = dir.join("locales");
        fs::create_dir_all(&locales).unwrap();
        fs::write(
            locales.join("en-US.ftl"),
            "r = { TEST($v, operator: \"+\", operand: 10) }\n",
        )
        .unwrap();

        let out = dir.join("out.rs");
        let test_def = ftl_builtin! {
            Test(FluentNum) {
                operator: String,
                operand: f64,
            } impl |this, out| {
                let result = *this.value + this.operand.unwrap_or(0.0);
                write!(out, "{}", result).unwrap();
            }
        };
        generator()
            .locales_dir(&locales)
            .default_lang("en-US")
            .output_path(&out)
            .register(test_def)
            .generate();

        let code = fs::read_to_string(&out).unwrap();
        assert!(
            code.contains("pub struct Test {"),
            "should emit Test struct"
        );
        assert!(code.contains("pub fn new"), "should emit new()");
        assert!(
            code.contains("pub fn operator"),
            "should emit operator builder"
        );
        assert!(code.contains("fn write_to"), "should emit write_to");
        assert!(
            code.contains("this.operand.unwrap_or"),
            "should include user body"
        );
        assert!(code.contains("v: Test"), "should use Test type in fn sig");

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
