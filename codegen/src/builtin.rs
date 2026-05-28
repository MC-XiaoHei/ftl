/// The `NUMBER()` built-in function impl
pub fn number() -> crate::BuiltInFuncDef {
    crate::ftl_builtin! {
        Number(FluentNum) {
            minimum_fraction_digits: i64,
            maximum_fraction_digits: i64,
            minimum_significant_digits: i64,
            maximum_significant_digits: i64,
            minimum_integer_digits: i64,
            use_grouping: bool,
            style: String,
            currency: String,
            currency_display: String,
        }

        impl |this, out, lang| {
            ftl_builtin::format_number(
                *this.value,
                this.minimum_fraction_digits,
                this.maximum_fraction_digits,
                this.minimum_significant_digits,
                this.maximum_significant_digits,
                this.minimum_integer_digits,
                this.use_grouping,
                this.style.as_deref(),
                this.currency.as_deref(),
                this.currency_display.as_deref(),
                out,
                &lang.into(),
            );
        }
    }
}

/// The `DATETIME()` built-in function impl
pub fn datetime() -> crate::BuiltInFuncDef {
    crate::ftl_builtin! {
        DateTime(FluentNum) {
            year: i64,
            month: i64,
            day: i64,
            hour: i64,
            minute: i64,
            second: i64,
            date_style: String,
            time_style: String,
            weekday: String,
            era: String,
            year_format: String,
            month_format: String,
            day_format: String,
            hour_format: String,
            minute_format: String,
            second_format: String,
            time_zone_name: String,
            hour12: bool,
            time_zone: String,
        }

        impl |this, out, lang| {
            ftl_builtin::format_datetime(
                this.year,
                this.month,
                this.day,
                this.hour,
                this.minute,
                this.second,
                this.date_style.as_deref(),
                this.time_style.as_deref(),
                this.weekday.as_deref(),
                this.era.as_deref(),
                this.year_format.as_deref(),
                this.month_format.as_deref(),
                this.day_format.as_deref(),
                this.hour_format.as_deref(),
                this.minute_format.as_deref(),
                this.second_format.as_deref(),
                this.time_zone_name.as_deref(),
                this.hour12,
                this.time_zone.as_deref(),
                out,
                &lang.into(),
            );
        }
    }
}
