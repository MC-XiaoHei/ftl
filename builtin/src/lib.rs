#![cfg_attr(coverage, feature(coverage_attribute))]

mod cldr_generated;
pub mod datetime;
pub mod number;

pub use datetime::format_datetime;
pub use number::format_number;
