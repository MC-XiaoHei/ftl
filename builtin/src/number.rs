use crate::cldr_generated;
use fixed_decimal::{Decimal, FloatPrecision};
use icu_decimal::options::{DecimalFormatterOptions, GroupingStrategy};
use icu_decimal::{self, DecimalFormatter};
use std::fmt::Write;
use writeable::Writeable;

pub fn format_number(
    value: f64,
    min_fraction_digits: Option<i64>,
    max_fraction_digits: Option<i64>,
    min_significant_digits: Option<i64>,
    max_significant_digits: Option<i64>,
    min_integer_digits: Option<i64>,
    use_grouping: Option<bool>,
    style: Option<&str>,
    currency: Option<&str>,
    currency_display: Option<&str>,
    out: &mut String,
    locale: &unic_langid::LanguageIdentifier,
) {
    if value.is_nan() {
        out.push_str("NaN");
        return;
    }
    if value.is_infinite() {
        if value.is_sign_negative() {
            out.push_str("-∞");
        } else {
            out.push_str("∞");
        }
        return;
    }

    let style = style.unwrap_or("decimal");
    match style {
        "percent" => format_percent(value, locale, out),
        "currency" => format_currency(value, currency, currency_display, locale, out),
        _ => format_decimal(
            value,
            min_fraction_digits,
            max_fraction_digits,
            min_significant_digits,
            max_significant_digits,
            min_integer_digits,
            use_grouping,
            locale,
            out,
        ),
    }
}

fn format_decimal(
    value: f64,
    min_fraction_digits: Option<i64>,
    max_fraction_digits: Option<i64>,
    min_significant_digits: Option<i64>,
    max_significant_digits: Option<i64>,
    min_integer_digits: Option<i64>,
    use_grouping: Option<bool>,
    locale: &unic_langid::LanguageIdentifier,
    out: &mut String,
) {
    let mut dec = f64_to_decimal(
        value,
        min_fraction_digits,
        max_fraction_digits,
        min_significant_digits,
        max_significant_digits,
    );
    if let Some(mid) = min_integer_digits.filter(|&m| m > 1) {
        let upper = *dec.absolute.magnitude_range().end();
        let target = mid as i16;
        if upper < target - 1 {
            dec.absolute.pad_start(target);
        }
    }

    let mut opts = DecimalFormatterOptions::default();
    if let Some(ug) = use_grouping {
        opts.grouping_strategy = Some(if ug {
            GroupingStrategy::Auto
        } else {
            GroupingStrategy::Never
        });
    }
    let fmt = DecimalFormatter::try_new(to_prefs(locale), opts)
        .expect("compiled_data should always be available");
    let _ = fmt.format(&dec).write_to(out);
}

fn format_percent(value: f64, locale: &unic_langid::LanguageIdentifier, out: &mut String) {
    let dec = f64_to_decimal(value * 100.0, None, None, None, None);
    let fmt = DecimalFormatter::try_new(to_prefs(locale), Default::default())
        .expect("compiled_data should always be available");
    let _ = fmt.format(&dec).write_to(out);

    let s = locale.to_string();
    let pct = cldr_generated::lookup(&s)
        .or_else(|| {
            s.split_once('-')
                .and_then(|(b, _)| cldr_generated::lookup(b))
        })
        .and_then(|d| extract_pattern_suffix(&d.percent_format))
        .unwrap_or("%");
    out.push_str(pct);
}

fn format_currency(
    value: f64,
    currency: Option<&str>,
    currency_display: Option<&str>,
    locale: &unic_langid::LanguageIdentifier,
    out: &mut String,
) {
    let cldr = resolve_cldr(locale);
    let dec = f64_to_decimal(value, Some(2), Some(2), None, None);
    let fmt = DecimalFormatter::try_new(to_prefs(locale), Default::default())
        .expect("compiled_data should always be available");
    let mut num_buf = String::new();
    let _ = fmt.format(&dec).write_to(&mut num_buf);

    let curr = currency.unwrap_or("");
    let display = currency_display.unwrap_or("symbol");
    let (symbol, name) = cldr
        .and_then(|d| d.currencies.iter().find(|(c, _)| *c == curr))
        .map(|(_, e)| (e.symbol, e.name))
        .unwrap_or((curr, curr));
    let placement = cldr
        .and_then(|d| extract_currency_placement(&d.currency_format))
        .unwrap_or(CurrencyPlacement::Prefix);

    match display {
        "code" => write_currency(out, curr, &num_buf, placement),
        "name" => write_currency(out, name, &num_buf, placement),
        _ => write_currency(out, symbol, &num_buf, placement),
    }
}

fn write_currency(out: &mut String, label: &str, number: &str, placement: CurrencyPlacement) {
    let sep = if label.chars().count() <= 1 { "" } else { " " };
    match placement {
        CurrencyPlacement::Prefix => {
            let _ = write!(out, "{}{}{}", label, sep, number);
        }
        CurrencyPlacement::Suffix => {
            let _ = write!(out, "{}{}{}", number, sep, label);
        }
    }
}

enum CurrencyPlacement {
    Prefix,
    Suffix,
}

fn extract_currency_placement(pattern: &str) -> Option<CurrencyPlacement> {
    if let Some(pos) = pattern.find('\u{00a4}') {
        Some(if pos == 0 {
            CurrencyPlacement::Prefix
        } else {
            CurrencyPlacement::Suffix
        })
    } else {
        None
    }
}

fn extract_pattern_suffix(pattern: &str) -> Option<&str> {
    let num_end = pattern.rfind(|c: char| c == '0' || c == '#' || c == ',' || c == '.')?;
    let suffix = &pattern[num_end + 1..];
    if suffix.is_empty() {
        None
    } else {
        Some(suffix)
    }
}

fn f64_to_decimal(
    value: f64,
    min_frac: Option<i64>,
    max_frac: Option<i64>,
    min_sig: Option<i64>,
    max_sig: Option<i64>,
) -> Decimal {
    let mut dec = Decimal::try_from_f64(value, FloatPrecision::RoundTrip)
        .expect("finite f64 always converts to Decimal");

    if let Some(mx) = max_sig {
        if !dec.absolute.magnitude_range().is_empty() {
            let upper = *dec.absolute.magnitude_range().end();
            dec = dec.rounded(upper - (mx as i16) + 1);
        }
        return dec;
    }
    if let Some(mn) = min_sig.filter(|&m| m > 0) {
        let current = significant_digits(&dec);
        if current < mn as i16 {
            pad_to_sig_digits(&mut dec, mn as i16);
        }
        return dec;
    }

    if let Some(mx) = max_frac {
        dec = dec.rounded(-mx as i16);
    }
    if let Some(mn) = min_frac.filter(|&m| m > 0) {
        dec.absolute.pad_end(-mn as i16);
    }
    dec
}

fn significant_digits(d: &Decimal) -> i16 {
    let upper = *d.absolute.magnitude_range().end();
    let lower = *d.absolute.magnitude_range().start();
    upper - lower + 1
}

fn pad_to_sig_digits(dec: &mut Decimal, target: i16) {
    let current = significant_digits(dec);
    let need = target - current;
    if need > 0 {
        let lower = *dec.absolute.magnitude_range().start();
        dec.absolute.pad_end(lower - need);
    }
}

fn to_prefs(locale: &unic_langid::LanguageIdentifier) -> icu_decimal::DecimalFormatterPreferences {
    let s = locale.to_string();
    s.parse::<icu_locale_core::Locale>()
        .ok()
        .map(|l| l.into())
        .unwrap_or_default()
}

fn resolve_cldr(
    locale: &unic_langid::LanguageIdentifier,
) -> Option<&'static cldr_generated::LocaleData> {
    let s = locale.to_string();
    cldr_generated::lookup(&s).or_else(|| {
        s.split_once('-')
            .and_then(|(base, _)| cldr_generated::lookup(base))
    })
}

#[cfg(test)]
#[cfg_attr(coverage, coverage(off))]
mod tests {
    use super::*;
    use unic_langid::LanguageIdentifier;

    fn en() -> LanguageIdentifier {
        "en".parse().unwrap()
    }
    fn de() -> LanguageIdentifier {
        "de".parse().unwrap()
    }
    fn locale(s: &str) -> LanguageIdentifier {
        s.parse().unwrap()
    }

    fn fmt(value: f64, opts: &[(&str, &str)], loc: &LanguageIdentifier) -> String {
        let mut out = String::new();
        let (mut min_frac, mut max_frac, mut min_sig, mut max_sig, mut min_int) =
            (None, None, None, None, None);
        let (mut grouping, mut style, mut curr, mut curr_disp) = (None, None, None, None);
        for (k, v) in opts {
            match *k {
                "min_frac" => min_frac = Some(v.parse().unwrap()),
                "max_frac" => max_frac = Some(v.parse().unwrap()),
                "min_sig" => min_sig = Some(v.parse().unwrap()),
                "max_sig" => max_sig = Some(v.parse().unwrap()),
                "min_int" => min_int = Some(v.parse().unwrap()),
                "grouping" => grouping = Some(*v == "true"),
                "style" => style = Some(*v),
                "currency" => curr = Some(*v),
                "curr_disp" => curr_disp = Some(*v),
                _ => {}
            }
        }
        format_number(
            value, min_frac, max_frac, min_sig, max_sig, min_int, grouping, style, curr, curr_disp,
            &mut out, loc,
        );
        out
    }

    #[test]
    fn decimal_default() {
        assert_eq!(fmt(1234.0, &[], &en()), "1,234");
    }
    #[test]
    fn decimal_no_grouping() {
        assert_eq!(fmt(1234.0, &[("grouping", "false")], &en()), "1234");
    }
    #[test]
    fn decimal_fraction_fixed() {
        assert_eq!(
            fmt(1.5, &[("min_frac", "3"), ("max_frac", "3")], &en()),
            "1.500"
        );
    }
    #[test]
    fn decimal_fraction_rounded() {
        assert_eq!(fmt(1.56, &[("max_frac", "1")], &en()), "1.6");
    }
    #[test]
    fn decimal_fraction_clipped() {
        assert_eq!(fmt(3.14159, &[("max_frac", "2")], &en()), "3.14");
    }
    #[test]
    fn decimal_zero() {
        assert_eq!(fmt(0.0, &[], &en()), "0");
    }
    #[test]
    fn decimal_negative() {
        assert_eq!(fmt(-42.0, &[], &en()), "-42");
    }
    #[test]
    fn decimal_large() {
        assert_eq!(fmt(1_000_000.0, &[], &en()), "1,000,000");
    }
    #[test]
    fn decimal_de_locale() {
        assert_eq!(fmt(1234.5, &[], &de()), "1.234,5");
    }
    #[test]
    fn decimal_ja_locale() {
        assert_eq!(fmt(1234.0, &[], &locale("ja")), "1,234");
    }
    #[test]
    fn decimal_nan() {
        assert_eq!(fmt(f64::NAN, &[], &en()), "NaN");
    }
    #[test]
    fn decimal_neg_inf() {
        assert_eq!(fmt(f64::NEG_INFINITY, &[], &en()), "-∞");
    }
    #[test]
    fn decimal_pos_inf() {
        assert_eq!(fmt(f64::INFINITY, &[], &en()), "∞");
    }
    #[test]
    fn decimal_grouping_auto() {
        assert_eq!(fmt(1234.0, &[("grouping", "true")], &en()), "1,234");
    }
    #[test]
    fn significant_max() {
        assert_eq!(fmt(12345.6789, &[("max_sig", "3")], &en()), "12,300");
    }
    #[test]
    fn significant_min_pads() {
        assert_eq!(fmt(5.0, &[("min_sig", "3")], &en()), "5.00");
    }
    #[test]
    fn fraction_range() {
        assert_eq!(
            fmt(3.1, &[("min_frac", "2"), ("max_frac", "4")], &en()),
            "3.10"
        );
    }
    #[test]
    fn fraction_range_full() {
        assert_eq!(
            fmt(3.14159, &[("min_frac", "2"), ("max_frac", "4")], &en()),
            "3.1416"
        );
    }
    #[test]
    fn min_int_pads() {
        assert_eq!(fmt(5.0, &[("min_int", "3")], &en()), "005");
    }
    #[test]
    fn min_int_noop() {
        assert_eq!(fmt(500.0, &[("min_int", "3")], &en()), "500");
    }

    #[test]
    fn percent_basic() {
        assert_eq!(fmt(0.25, &[("style", "percent")], &en()), "25%");
    }
    #[test]
    fn percent_whole() {
        assert_eq!(fmt(1.0, &[("style", "percent")], &en()), "100%");
    }
    #[test]
    fn percent_above_100() {
        assert_eq!(fmt(2.5, &[("style", "percent")], &en()), "250%");
    }
    #[test]
    fn percent_arabic() {
        let r = fmt(0.5, &[("style", "percent")], &locale("ar"));
        assert!(r.contains('٪') || r.contains('%'), "got: {r}");
    }
    #[test]
    fn percent_fallback() {
        assert_eq!(fmt(0.5, &[("style", "percent")], &locale("xx-YY")), "50%");
    }

    #[test]
    fn currency_symbol() {
        assert_eq!(
            fmt(12.34, &[("style", "currency"), ("currency", "USD")], &en()),
            "$12.34"
        );
    }
    #[test]
    fn currency_code() {
        assert_eq!(
            fmt(
                5.0,
                &[
                    ("style", "currency"),
                    ("currency", "USD"),
                    ("curr_disp", "code")
                ],
                &en()
            ),
            "USD 5.00"
        );
    }
    #[test]
    fn currency_unknown() {
        assert_eq!(
            fmt(9.99, &[("style", "currency"), ("currency", "XYZ")], &en()),
            "XYZ 9.99"
        );
    }
    #[test]
    fn currency_no_currency() {
        assert_eq!(fmt(9.99, &[("style", "currency")], &en()), "9.99");
    }
    #[test]
    fn currency_fallback() {
        assert_eq!(
            fmt(
                3.50,
                &[("style", "currency"), ("currency", "EUR")],
                &locale("en-XY")
            ),
            "€3.50"
        );
    }
    #[test]
    fn currency_suffix_fr() {
        let r = fmt(
            12.34,
            &[("style", "currency"), ("currency", "EUR")],
            &locale("fr"),
        );
        assert!(r.contains("€") || r.contains("EUR"), "got: {r}");
    }
    #[test]
    fn currency_name() {
        let r = fmt(
            1.0,
            &[
                ("style", "currency"),
                ("currency", "USD"),
                ("curr_disp", "name"),
            ],
            &en(),
        );
        assert!(
            r.starts_with("US Dollar") || r.contains("US Dollar"),
            "got: {r}"
        );
    }

    #[test]
    fn sig_digits_max() {
        let d = f64_to_decimal(123.456, None, None, None, Some(3));
        assert_eq!(d.to_string(), "123");
    }
    #[test]
    fn sig_digits_min_pad() {
        let d = f64_to_decimal(5.0, None, None, Some(3), None);
        assert_eq!(d.to_string(), "5.00");
    }
    #[test]
    fn sig_digits_min_zero() {
        let d = f64_to_decimal(0.0, None, None, Some(3), None);
        assert_eq!(d.to_string(), "0.00");
    }
    #[test]
    fn frac_exact() {
        let d = f64_to_decimal(3.14, Some(2), Some(2), None, None);
        assert_eq!(d.to_string(), "3.14");
    }
    #[test]
    fn frac_min_only() {
        let d = f64_to_decimal(3.0, Some(3), None, None, None);
        assert_eq!(d.to_string(), "3.000");
    }

    #[test]
    fn currency_prefix_placement() {
        assert!(matches!(
            extract_currency_placement("¤#,##0.00"),
            Some(CurrencyPlacement::Prefix)
        ));
    }
    #[test]
    fn currency_suffix_placement() {
        assert!(matches!(
            extract_currency_placement("#,##0.00¤"),
            Some(CurrencyPlacement::Suffix)
        ));
    }
    #[test]
    fn currency_no_placement() {
        assert!(extract_currency_placement("#,##0.00").is_none());
    }
    #[test]
    fn pct_suffix() {
        assert_eq!(extract_pattern_suffix("#,##0%"), Some("%"));
    }
    #[test]
    fn pct_no_suffix() {
        assert_eq!(extract_pattern_suffix("#,##0"), None);
    }
    #[test]
    fn pct_only_symbol() {
        assert_eq!(extract_pattern_suffix("%%%"), None);
    }
}
