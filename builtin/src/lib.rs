#![cfg_attr(coverage, feature(coverage_attribute))]

#[allow(missing_docs, non_upper_case_globals, dead_code)]
mod cldr_generated {
    include!(concat!(env!("OUT_DIR"), "/cldr_generated.rs"));
}
pub mod datetime;
pub mod number;

pub use datetime::format_datetime;
pub use number::format_number;
