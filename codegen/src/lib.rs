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

#[derive(Clone, Debug, PartialEq)]
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
#[macro_export]
macro_rules! ftl_builtin {
    (
        $name:ident ( $base:ty ) {
            $( $arg:ident : $ty:tt ),* $(,)?
        }
    ) => {
        $crate::BuiltInFuncDef {
            name: stringify!($name).to_uppercase(),
            ty_name: stringify!($name).to_string(),
            named_args: vec![
                $( $crate::__builtin_named_arg!($arg, $ty) ),*
            ],
            write_to_body: None,
        }
    };
    (
        $name:ident ( $base:ty ) {
            $( $arg:ident : $ty:tt ),* $(,)?
        }
        impl |$this:ident, $out:ident $(, $lang:ident)?| $body:block
    ) => {
        $crate::BuiltInFuncDef {
            name: stringify!($name).to_uppercase(),
            ty_name: stringify!($name).to_string(),
            named_args: vec![
                $( $crate::__builtin_named_arg!($arg, $ty) ),*
            ],
            write_to_body: Some(stringify!($body).to_string()),
        }
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
        self.builtins.insert(def.name.clone(), def);
        self
    }

    /// Collect registered builtins, always including NUMBER and DATETIME.
    fn get_builtins(&self) -> BTreeMap<String, BuiltInFuncDef> {
        let mut builtins = self.builtins.clone();
        // NUMBER and DATETIME are always available — fmt.rs handles them directly.
        builtins
            .entry("NUMBER".to_string())
            .or_insert_with(|| BuiltInFuncDef {
                name: "NUMBER".to_string(),
                ty_name: "Number".to_string(),
                named_args: Vec::new(),
                write_to_body: None,
            });
        builtins
            .entry("DATETIME".to_string())
            .or_insert_with(|| BuiltInFuncDef {
                name: "DATETIME".to_string(),
                ty_name: "DateTime".to_string(),
                named_args: Vec::new(),
                write_to_body: None,
            });
        builtins
    }

    /// Generate the i18n Rust module.
    pub fn generate(&self) {
        let builtins = self.get_builtins();
        let output_path = self.get_output_path();
        let gen = parse::Generator::load(
            &self.locales_dir,
            &self.primary,
            &self.module_path,
            &builtins,
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
mod tests;

#[cfg(test)]
#[cfg_attr(coverage, coverage(off))]
mod lib_tests {
    use super::*;

    #[test]
    fn snake_to_camel_basic() {
        assert_eq!(__snake_to_camel("foo_bar"), "fooBar");
    }

    #[test]
    fn snake_to_camel_single() {
        assert_eq!(__snake_to_camel("foo"), "foo");
    }

    #[test]
    fn snake_to_camel_consecutive_underscores() {
        assert_eq!(__snake_to_camel("foo__bar"), "fooBar");
    }

    #[test]
    fn snake_to_camel_trailing_underscore() {
        assert_eq!(__snake_to_camel("foo_"), "foo");
    }

    #[test]
    fn snake_to_camel_leading_underscore() {
        assert_eq!(__snake_to_camel("_foo"), "Foo");
    }

    #[test]
    fn snake_to_camel_empty() {
        assert_eq!(__snake_to_camel(""), "");
    }

    #[test]
    fn snake_to_camel_camel_already() {
        assert_eq!(__snake_to_camel("fooBar"), "fooBar");
    }

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

    #[test]
    fn get_builtins_always_includes_number_and_datetime() {
        let builtins = generator().get_builtins();
        assert!(builtins.contains_key("NUMBER"));
        assert!(builtins.contains_key("DATETIME"));
    }

    #[test]
    fn register_custom_builtin() {
        let def = BuiltInFuncDef {
            name: "CUSTOM".to_string(),
            ty_name: "Custom".to_string(),
            named_args: vec![],
            write_to_body: Some("write!(out, \"custom\").unwrap();".to_string()),
        };
        let builtins = generator().register(def).get_builtins();
        assert!(builtins.contains_key("CUSTOM"));
    }

    #[test]
    fn module_path_appears_in_generated_code() {
        let dir = std::path::PathBuf::from(std::env::temp_dir())
            .join(format!("ftl_modpath_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let locales = dir.join("locales");
        fs::create_dir_all(&locales).unwrap();
        fs::write(locales.join("en-US.ftl"), "x = 1\n").unwrap();

        let out = dir.join("out.rs");
        generator()
            .locales_dir(&locales)
            .default_lang("en-US")
            .module_path("i18n")
            .output_path(&out)
            .generate();

        let code = fs::read_to_string(&out).unwrap();
        assert!(code.contains("$crate::i18n"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn ftl_builtin_macro_with_body() {
        let def = ftl_builtin! {
            Html(f64) {
                class: String,
                count: i64,
                ratio: f64,
                enabled: bool,
            }
            impl |this, out| {
                write!(out, "<div>\"{}\"</div>", this.value).unwrap();
            }
        };
        assert_eq!(def.name, "HTML");
        assert_eq!(def.ty_name, "Html");
        assert!(def.write_to_body.is_some());
        assert_eq!(def.named_args.len(), 4);
        assert_eq!(def.named_args[0].ftl_name, "class");
        assert_eq!(def.named_args[1].ftl_name, "count");
        assert_eq!(def.named_args[2].ftl_name, "ratio");
        assert_eq!(def.named_args[3].ftl_name, "enabled");
    }

    #[test]
    fn ftl_builtin_macro_without_body() {
        let def = ftl_builtin! {
            Bold(f64) {
                level: i64,
            }
        };
        assert_eq!(def.name, "BOLD");
        assert_eq!(def.ty_name, "Bold");
        assert!(def.write_to_body.is_none());
        assert_eq!(def.named_args.len(), 1);
        assert_eq!(def.named_args[0].ftl_name, "level");
        assert_eq!(def.named_args[0].arg_type, BuiltInArgType::Int);
    }
}
