#![cfg_attr(coverage, feature(coverage_attribute))]

mod cldr_generated {
    include!(concat!(env!("OUT_DIR"), "/cldr_generated.rs"));
}
pub mod datetime;
pub mod number;

use std::borrow::Cow;
use std::sync::OnceLock;
pub use unic_langid;
use unic_langid::LanguageIdentifier;

pub struct FluentNum(pub(crate) f64);

macro_rules! impl_from_for_fluentnum {
    ($($ty:ty),* $(,)?) => {
        $(impl From<$ty> for FluentNum {
            fn from(v: $ty) -> Self { Self(v as f64) }
        })*
    };
}
impl_from_for_fluentnum!(usize, u64, u32, u16, u8, i64, i32, i16, i8, isize);
impl From<f64> for FluentNum {
    fn from(v: f64) -> Self {
        Self(v)
    }
}
impl From<f32> for FluentNum {
    fn from(v: f32) -> Self {
        Self(v as f64)
    }
}

impl std::ops::Deref for FluentNum {
    type Target = f64;
    fn deref(&self) -> &f64 {
        &self.0
    }
}

impl core::fmt::Display for FluentNum {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub enum FluentArg<'a> {
    Str(Cow<'a, str>),
    Num(FluentNum),
}

impl FluentArg<'_> {
    pub fn write_localized(&self, s: &mut String) {
        match self {
            FluentArg::Str(v) => s.push_str(v),
            FluentArg::Num(v) => number::format(
                **v,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                s,
                &locale(),
            ),
        }
    }
}

impl<'a> From<&'a str> for FluentArg<'a> {
    fn from(v: &'a str) -> Self {
        FluentArg::Str(Cow::Borrowed(v))
    }
}
impl From<String> for FluentArg<'_> {
    fn from(v: String) -> Self {
        FluentArg::Str(Cow::Owned(v))
    }
}

macro_rules! impl_from_for_fluentarg_num {
    ($($ty:ty),* $(,)?) => {
        $(impl From<$ty> for FluentArg<'_> {
            fn from(v: $ty) -> Self { FluentArg::Num(FluentNum::from(v)) }
        })*
    };
}
impl_from_for_fluentarg_num!(usize, u64, u32, u16, u8, i64, i32, i16, i8, isize, f64, f32);
impl From<FluentNum> for FluentArg<'_> {
    fn from(v: FluentNum) -> Self {
        FluentArg::Num(v)
    }
}

pub trait Localizable<'a> {
    fn to_fluent_arg(self) -> FluentArg<'a>;
}

impl<'a, T: Into<FluentArg<'a>>> Localizable<'a> for T {
    fn to_fluent_arg(self) -> FluentArg<'a> {
        self.into()
    }
}

static FORMAT_LOCALE: OnceLock<std::sync::Mutex<LanguageIdentifier>> = OnceLock::new();

fn format_locale() -> &'static std::sync::Mutex<LanguageIdentifier> {
    FORMAT_LOCALE.get_or_init(|| std::sync::Mutex::new("en-US".parse().unwrap()))
}

/// Set the formatting locale.
pub fn set_locale(lang: &LanguageIdentifier) {
    *format_locale().lock().unwrap() = lang.clone();
}

/// Get the current formatting locale.
pub fn locale() -> LanguageIdentifier {
    format_locale().lock().unwrap().clone()
}

#[cfg(test)]
#[cfg_attr(coverage, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn fluent_num_from_primitives() {
        assert!((*FluentNum::from(42u32) - 42.0).abs() < 1e-10);
        assert!((*FluentNum::from(42i64) - 42.0).abs() < 1e-10);
        assert!((*FluentNum::from(42.0f64) - 42.0).abs() < 1e-10);
        assert!((*FluentNum::from(42.0f32) - 42.0).abs() < 1e-10);
        assert!((*FluentNum::from(42usize) - 42.0).abs() < 1e-10);
    }

    #[test]
    fn fluent_num_display() {
        assert_eq!(format!("{}", FluentNum::from(42.5)), "42.5");
    }

    #[test]
    fn fluent_arg_from_str() {
        let arg: FluentArg<'_> = "hello".into();
        assert!(matches!(arg, FluentArg::Str(s) if s == "hello"));
    }

    #[test]
    fn fluent_arg_from_string() {
        let arg: FluentArg<'_> = String::from("world").into();
        assert!(matches!(arg, FluentArg::Str(s) if s == "world"));
    }

    #[test]
    fn fluent_arg_from_number() {
        let arg: FluentArg<'_> = 42i32.into();
        assert!(matches!(arg, FluentArg::Num(n) if (*n - 42.0).abs() < 1e-10));
    }

    #[test]
    fn fluent_arg_from_f64() {
        let arg: FluentArg<'_> = 3.14f64.into();
        assert!(matches!(arg, FluentArg::Num(n) if (*n - 3.14).abs() < 1e-10));
    }

    #[test]
    fn fluent_arg_from_fluent_num() {
        let fn_num = FluentNum::from(99.0);
        let arg: FluentArg<'_> = fn_num.into();
        assert!(matches!(arg, FluentArg::Num(n) if (*n - 99.0).abs() < 1e-10));
    }

    #[test]
    fn fluent_arg_write_localized_str() {
        let arg: FluentArg<'_> = "test".into();
        let mut s = String::new();
        arg.write_localized(&mut s);
        assert_eq!(s, "test");
    }

    #[test]
    fn fluent_arg_write_localized_num() {
        let arg: FluentArg<'_> = 42.0.into();
        let mut s = String::new();
        arg.write_localized(&mut s);
        assert!(!s.is_empty());
    }

    #[test]
    fn locale_set_and_get() {
        let loc: LanguageIdentifier = "de".parse().unwrap();
        set_locale(&loc);
        assert_eq!(locale(), loc);
        // reset for other tests
        let en: LanguageIdentifier = "en-US".parse().unwrap();
        set_locale(&en);
    }

    #[test]
    fn localizable_trait() {
        let val: FluentArg<'_> = 42.0.to_fluent_arg();
        assert!(matches!(val, FluentArg::Num(n) if (*n - 42.0).abs() < 1e-10));
    }
}
