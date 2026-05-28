use icu_calendar::{Date, Gregorian};
use icu_datetime::fieldsets::{zone, YMD, YMDE, YMDET, YMDT};
use icu_datetime::options::{Length, TimePrecision, YearStyle};
use icu_datetime::{self, DateTimeFormatter, FixedCalendarDateTimeFormatter};
use icu_time::zone::{models, TimeZoneInfo};
use icu_time::{DateTime, Time, TimeZone, ZonedDateTime};
use writeable::Writeable;

pub fn format_datetime(
    year: Option<i64>,
    month: Option<i64>,
    day: Option<i64>,
    hour: Option<i64>,
    minute: Option<i64>,
    second: Option<i64>,
    date_style: Option<&str>,
    time_style: Option<&str>,
    weekday: Option<&str>,
    era: Option<&str>,
    year_format: Option<&str>,
    month_format: Option<&str>,
    day_format: Option<&str>,
    hour_format: Option<&str>,
    minute_format: Option<&str>,
    second_format: Option<&str>,
    time_zone_name: Option<&str>,
    hour12: Option<bool>,
    time_zone: Option<&str>,
    out: &mut String,
    locale: &unic_langid::LanguageIdentifier,
) {
    let has_individual = year_format
        .or(month_format)
        .or(day_format)
        .or(weekday)
        .or(era)
        .or(hour_format)
        .or(minute_format)
        .or(second_format)
        .is_some();
    let has_time = hour.or(minute).or(second).is_some();
    let has_zone = time_zone_name.is_some() || time_zone.is_some();

    let date = match Date::try_new_gregorian(
        year.unwrap_or(0) as i32,
        month.unwrap_or(1) as u8,
        day.unwrap_or(1) as u8,
    ) {
        Ok(d) => d,
        Err(_) => return,
    };
    let prefs = resolve_prefs(locale, hour12);

    if has_individual {
        let has_wd = weekday.is_some();
        let mut length = pick_length(year_format, month_format, day_format, weekday);
        if let Some(ev) = era {
            if let ("long", Length::Short | Length::Medium) = (ev, length) {
                length = Length::Long;
            }
        }
        let yf = era.map(|s| match s {
            "long" => YearStyle::Full,
            "short" => YearStyle::WithEra,
            _ => YearStyle::Auto,
        });
        let tp = pick_time_precision(hour_format, minute_format, second_format);

        if has_time || hour_format.is_some() || minute_format.is_some() || second_format.is_some() {
            let time = match Time::try_new(
                hour.unwrap_or(0) as u8,
                minute.unwrap_or(0) as u8,
                second.unwrap_or(0) as u8,
                0,
            ) {
                Ok(t) => t,
                Err(_) => return,
            };
            let dt = DateTime { date, time };

            if has_zone {
                zoned(
                    dt,
                    has_wd,
                    length,
                    yf,
                    tp,
                    time_zone_name,
                    time_zone,
                    &prefs,
                    out,
                );
                return;
            }

            if has_wd {
                let mut fs = YMDET::for_length(length);
                if let Some(v) = yf {
                    fs.year_style = Some(v);
                }
                fs.time_precision = Some(tp);
                let fmt = DateTimeFormatter::try_new(prefs.clone(), fs)
                    .expect("compiled_data should always be available");
                let _ = fmt.format(&dt).write_to(out);
            } else {
                let mut fs = YMDT::for_length(length);
                if let Some(v) = yf {
                    fs.year_style = Some(v);
                }
                fs.time_precision = Some(tp);
                let fmt = DateTimeFormatter::try_new(prefs.clone(), fs)
                    .expect("compiled_data should always be available");
                let _ = fmt.format(&dt).write_to(out);
            }
        } else {
            if has_zone {
                zoned_without_time(
                    date,
                    has_wd,
                    length,
                    yf,
                    time_zone_name,
                    time_zone,
                    &prefs,
                    out,
                );
                return;
            }
            if has_wd {
                let mut fs = YMDE::for_length(length);
                if let Some(v) = yf {
                    fs.year_style = Some(v);
                }
                let fmt =
                    FixedCalendarDateTimeFormatter::<Gregorian, _>::try_new(prefs.clone(), fs)
                        .expect("compiled_data should always be available");
                let _ = fmt.format(&date).write_to(out);
            } else {
                let mut fs = YMD::for_length(length);
                if let Some(v) = yf {
                    fs.year_style = Some(v);
                }
                let fmt =
                    FixedCalendarDateTimeFormatter::<Gregorian, _>::try_new(prefs.clone(), fs)
                        .expect("compiled_data should always be available");
                let _ = fmt.format(&date).write_to(out);
            }
        }
    } else {
        let ds = date_style.unwrap_or("medium");
        let ts = time_style.unwrap_or("medium");
        if has_time || time_style.is_some() {
            let time = match Time::try_new(
                hour.unwrap_or(0) as u8,
                minute.unwrap_or(0) as u8,
                second.unwrap_or(0) as u8,
                0,
            ) {
                Ok(t) => t,
                Err(_) => return,
            };
            let dt = DateTime { date, time };

            if has_zone {
                format_zone_datetime(dt, ds, ts, time_zone_name, time_zone, &prefs, out);
                return;
            }
            format_style_with_time(dt, ds, ts, &prefs, out);
        } else {
            if has_zone {
                format_zone_date_only(date, ds, time_zone_name, time_zone, &prefs, out);
                return;
            }
            format_style_date_only(date, ds, &prefs, out);
        }
    }
}
fn zoned(
    dt: DateTime<Gregorian>,
    has_wd: bool,
    length: Length,
    yf: Option<YearStyle>,
    tp: TimePrecision,
    tz_name: Option<&str>,
    tz: Option<&str>,
    prefs: &icu_datetime::DateTimeFormatterPreferences,
    out: &mut String,
) {
    let tz_info = build_tz(tz).at_date_time(dt);
    let zd = ZonedDateTime {
        date: dt.date,
        time: dt.time,
        zone: tz_info,
    };
    if has_wd {
        let mut y = YMDET::for_length(length);
        if let Some(v) = yf {
            y.year_style = Some(v);
        }
        y.time_precision = Some(tp);
        if tz_name == Some("short") {
            let fmt = DateTimeFormatter::try_new(prefs.clone(), y.with_zone(zone::SpecificShort))
                .expect("compiled_data should always be available");
            let _ = fmt.format(&zd).write_to(out);
        } else {
            let fmt = DateTimeFormatter::try_new(prefs.clone(), y.with_zone(zone::SpecificLong))
                .expect("compiled_data should always be available");
            let _ = fmt.format(&zd).write_to(out);
        }
    } else {
        let mut y = YMDT::for_length(length);
        if let Some(v) = yf {
            y.year_style = Some(v);
        }
        y.time_precision = Some(tp);
        if tz_name == Some("short") {
            let fmt = DateTimeFormatter::try_new(prefs.clone(), y.with_zone(zone::SpecificShort))
                .expect("compiled_data should always be available");
            let _ = fmt.format(&zd).write_to(out);
        } else {
            let fmt = DateTimeFormatter::try_new(prefs.clone(), y.with_zone(zone::SpecificLong))
                .expect("compiled_data should always be available");
            let _ = fmt.format(&zd).write_to(out);
        }
    }
}

fn zoned_without_time(
    date: Date<Gregorian>,
    has_wd: bool,
    length: Length,
    yf: Option<YearStyle>,
    tz_name: Option<&str>,
    tz: Option<&str>,
    prefs: &icu_datetime::DateTimeFormatterPreferences,
    out: &mut String,
) {
    let tz_info = build_tz(tz);
    let time = Time::try_new(0, 0, 0, 0).expect("midnight time is always valid");
    let dt = DateTime { date, time };
    let zd = ZonedDateTime {
        date,
        time,
        zone: tz_info.at_date_time(dt),
    };
    if has_wd {
        let mut y = YMDE::for_length(length);
        if let Some(v) = yf {
            y.year_style = Some(v);
        }
        if tz_name == Some("short") {
            let fmt = DateTimeFormatter::try_new(prefs.clone(), y.with_zone(zone::SpecificShort))
                .expect("compiled_data should always be available");
            let _ = fmt.format(&zd).write_to(out);
        } else {
            let fmt = DateTimeFormatter::try_new(prefs.clone(), y.with_zone(zone::SpecificLong))
                .expect("compiled_data should always be available");
            let _ = fmt.format(&zd).write_to(out);
        }
    } else {
        let mut y = YMD::for_length(length);
        if let Some(v) = yf {
            y.year_style = Some(v);
        }
        if tz_name == Some("short") {
            let fmt = DateTimeFormatter::try_new(prefs.clone(), y.with_zone(zone::SpecificShort))
                .expect("compiled_data should always be available");
            let _ = fmt.format(&zd).write_to(out);
        } else {
            let fmt = DateTimeFormatter::try_new(prefs.clone(), y.with_zone(zone::SpecificLong))
                .expect("compiled_data should always be available");
            let _ = fmt.format(&zd).write_to(out);
        }
    }
}
fn format_zone_datetime(
    dt: DateTime<Gregorian>,
    ds: &str,
    ts: &str,
    tz_name: Option<&str>,
    tz: Option<&str>,
    prefs: &icu_datetime::DateTimeFormatterPreferences,
    out: &mut String,
) {
    let tz_base = build_tz(tz);
    let tz_at = tz_base.at_date_time(dt);
    let zd = ZonedDateTime {
        date: dt.date,
        time: dt.time,
        zone: tz_at,
    };
    let (length, tp) = match (ds, ts) {
        ("long", "long") => (Length::Long, TimePrecision::Second),
        ("long", _) => (Length::Long, TimePrecision::Minute),
        ("short", "short") => (Length::Short, TimePrecision::Minute),
        ("short", _) => (Length::Short, TimePrecision::Second),
        _ => (Length::Medium, TimePrecision::Second),
    };
    let mut y = YMDT::for_length(length);
    y.time_precision = Some(tp);
    match tz_name {
        Some("short") => {
            let fmt = DateTimeFormatter::try_new(prefs.clone(), y.with_zone(zone::SpecificShort))
                .expect("compiled_data should always be available");
            let _ = fmt.format(&zd).write_to(out);
        }
        _ => {
            let fmt = DateTimeFormatter::try_new(prefs.clone(), y.with_zone(zone::SpecificLong))
                .expect("compiled_data should always be available");
            let _ = fmt.format(&zd).write_to(out);
        }
    }
}

fn format_zone_date_only(
    date: Date<Gregorian>,
    ds: &str,
    tz_name: Option<&str>,
    tz: Option<&str>,
    prefs: &icu_datetime::DateTimeFormatterPreferences,
    out: &mut String,
) {
    let tz_base = build_tz(tz);
    let time = Time::try_new(0, 0, 0, 0).expect("midnight time is always valid");
    let dt = DateTime { date, time };
    let tz_at = tz_base.at_date_time(dt);
    let zd = ZonedDateTime {
        date,
        time,
        zone: tz_at,
    };
    let y = match ds {
        "long" => YMD::long(),
        "short" => YMD::short(),
        _ => YMD::medium(),
    };
    match tz_name {
        Some("short") => {
            let fmt = DateTimeFormatter::try_new(prefs.clone(), y.with_zone(zone::SpecificShort))
                .expect("compiled_data should always be available");
            let _ = fmt.format(&zd).write_to(out);
        }
        _ => {
            let fmt = DateTimeFormatter::try_new(prefs.clone(), y.with_zone(zone::SpecificLong))
                .expect("compiled_data should always be available");
            let _ = fmt.format(&zd).write_to(out);
        }
    }
}

fn build_tz(tz: Option<&str>) -> TimeZoneInfo<models::Base> {
    match tz {
        Some(id) => TimeZone::from_iana_id(id).without_offset(),
        None => TimeZoneInfo::utc(),
    }
}

#[cfg(test)]
#[cfg_attr(coverage, coverage(off))]
mod tests {
    use super::*;
    use unic_langid::LanguageIdentifier;

    fn en() -> LanguageIdentifier {
        "en".parse().unwrap()
    }
    fn locale(s: &str) -> LanguageIdentifier {
        s.parse().unwrap()
    }

    fn fmt(
        args: &[(&str, i64)],
        ds: Option<&str>,
        ts: Option<&str>,
        loc: &LanguageIdentifier,
    ) -> String {
        let mut out = String::new();
        let (mut y, mut m, mut d, mut hh, mut mm, mut ss) = (None, None, None, None, None, None);
        for &(k, v) in args {
            match k {
                "year" => y = Some(v),
                "month" => m = Some(v),
                "day" => d = Some(v),
                "hour" => hh = Some(v),
                "min" => mm = Some(v),
                "sec" => ss = Some(v),
                _ => {}
            }
        }
        format_datetime(
            y, m, d, hh, mm, ss, ds, ts, None, None, None, None, None, None, None, None, None,
            None, None, &mut out, loc,
        );
        out
    }

    #[test]
    fn date_medium() {
        let r = fmt(
            &[("year", 2024), ("month", 5), ("day", 17)],
            None,
            None,
            &en(),
        );
        assert!(r.contains("May"), "got: {r}");
    }
    #[test]
    fn date_long() {
        let r = fmt(
            &[("year", 2024), ("month", 5), ("day", 17)],
            Some("long"),
            None,
            &en(),
        );
        assert!(r.contains("May"), "got: {r}");
    }
    #[test]
    fn date_short() {
        let r = fmt(
            &[("year", 2024), ("month", 5), ("day", 17)],
            Some("short"),
            None,
            &en(),
        );
        assert!(r.contains("24"), "got: {r}");
    }
    #[test]
    fn date_invalid() {
        let mut out = String::new();
        format_datetime(
            Some(2024),
            Some(2),
            Some(30),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            &mut out,
            &en(),
        );
        assert_eq!(out, "");
    }
    #[test]
    fn date_de() {
        let r = fmt(
            &[("year", 2024), ("month", 12), ("day", 25)],
            Some("long"),
            None,
            &locale("de"),
        );
        assert!(r.contains("Dezember"), "got: {r}");
    }
    #[test]
    fn datetime_default() {
        let r = fmt(
            &[
                ("year", 2024),
                ("month", 5),
                ("day", 17),
                ("hour", 14),
                ("min", 30),
            ],
            None,
            None,
            &en(),
        );
        assert!(r.contains("2024"), "got: {r}");
    }
    #[test]
    fn datetime_long_long() {
        let r = fmt(
            &[
                ("year", 2024),
                ("month", 5),
                ("day", 17),
                ("hour", 14),
                ("min", 30),
                ("sec", 5),
            ],
            Some("long"),
            Some("long"),
            &en(),
        );
        assert!(r.contains("2024"), "got: {r}");
    }
    #[test]
    fn datetime_bad_time() {
        let mut out = String::new();
        format_datetime(
            Some(2024),
            Some(5),
            Some(17),
            Some(99),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            &mut out,
            &en(),
        );
        assert_eq!(out, "");
    }
    #[test]
    fn hour_triggers_time() {
        let r = fmt(
            &[("year", 2024), ("month", 5), ("day", 17), ("hour", 9)],
            None,
            None,
            &en(),
        );
        assert!(r.contains("9"), "got: {r}");
    }
    #[test]
    fn to_prefs_ok() {
        let loc = "de".parse().unwrap();
        let p = to_prefs(&loc);
        assert!(DateTimeFormatter::try_new(p, YMD::medium()).is_ok());
    }

    #[test]
    fn ind_month_long() {
        let mut out = String::new();
        format_datetime(
            Some(2024),
            Some(5),
            Some(17),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some("long"),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            &mut out,
            &en(),
        );
        assert!(out.contains("May"), "got: {out}");
    }
    #[test]
    fn ind_year_2digit() {
        let mut out = String::new();
        format_datetime(
            Some(2024),
            Some(5),
            Some(17),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some("2-digit"),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            &mut out,
            &en(),
        );
        assert!(out.contains("24"), "got: {out}");
    }
    #[test]
    fn ind_weekday() {
        let mut out = String::new();
        format_datetime(
            Some(2024),
            Some(5),
            Some(17),
            None,
            None,
            None,
            None,
            None,
            Some("long"),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            &mut out,
            &en(),
        );
        assert!(out.contains("Fri") || out.contains("Friday"), "got: {out}");
    }
    #[test]
    fn ind_no_params() {
        let mut out = String::new();
        format_datetime(
            Some(2024),
            Some(5),
            Some(17),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            &mut out,
            &en(),
        );
        assert!(out.contains("May"), "got: {out}");
    }
    #[test]
    fn ind_weekday_time() {
        let mut out = String::new();
        format_datetime(
            Some(2024),
            Some(5),
            Some(17),
            Some(14),
            Some(30),
            None,
            None,
            None,
            Some("short"),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            &mut out,
            &en(),
        );
        assert!(out.contains("Fri"), "got: {out}");
    }
    #[test]
    fn ind_second_precision() {
        let mut out = String::new();
        format_datetime(
            Some(2024),
            Some(5),
            Some(17),
            Some(14),
            Some(30),
            Some(45),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some("numeric"),
            None,
            None,
            None,
            &mut out,
            &en(),
        );
        assert!(out.contains("45"), "got: {out}");
    }
    #[test]
    fn ind_era_long() {
        let mut out = String::new();
        format_datetime(
            Some(2024),
            Some(5),
            Some(17),
            None,
            None,
            None,
            None,
            None,
            None,
            Some("long"),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            &mut out,
            &en(),
        );
        assert!(out.contains("AD") || out.contains("May"), "got: {out}");
    }
    #[test]
    fn ind_era_short() {
        let mut out = String::new();
        format_datetime(
            Some(2024),
            Some(5),
            Some(17),
            None,
            None,
            None,
            None,
            None,
            None,
            Some("short"),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            &mut out,
            &en(),
        );
        assert!(out.contains("2024"), "got: {out}");
    }
    #[test]
    fn ind_era_narrow() {
        let mut out = String::new();
        format_datetime(
            Some(2024),
            Some(5),
            Some(17),
            None,
            None,
            None,
            None,
            None,
            None,
            Some("narrow"),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            &mut out,
            &en(),
        );
        assert!(out.contains("2024"), "got: {out}");
    }
    #[test]
    fn ind_bad_time_wd() {
        let mut out = String::new();
        format_datetime(
            Some(2024),
            Some(5),
            Some(17),
            Some(99),
            None,
            None,
            None,
            None,
            Some("long"),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            &mut out,
            &en(),
        );
        assert_eq!(out, "");
    }
    #[test]
    fn ind_time_no_wd() {
        let mut out = String::new();
        format_datetime(
            Some(2024),
            Some(5),
            Some(17),
            Some(10),
            Some(20),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            &mut out,
            &en(),
        );
        assert!(out.contains("10"), "got: {out}");
    }
    #[test]
    fn style_date_long() {
        let mut out = String::new();
        format_datetime(
            Some(2024),
            Some(5),
            Some(17),
            None,
            None,
            None,
            Some("long"),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            &mut out,
            &en(),
        );
        assert!(out.contains("May"), "got: {out}");
    }
    #[test]
    fn style_time_short() {
        let mut out = String::new();
        format_datetime(
            Some(2024),
            Some(5),
            Some(17),
            Some(14),
            Some(30),
            None,
            Some("long"),
            Some("short"),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            &mut out,
            &en(),
        );
        assert!(out.contains("PM") || out.contains("14"), "got: {out}");
    }
    #[test]
    fn style_short_short() {
        let mut out = String::new();
        format_datetime(
            Some(2024),
            Some(5),
            Some(17),
            Some(14),
            Some(30),
            None,
            Some("short"),
            Some("short"),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            &mut out,
            &en(),
        );
        assert!(
            out.contains("24") && (out.contains("PM") || out.contains("14")),
            "got: {out}"
        );
    }
    #[test]
    fn style_short_med() {
        let mut out = String::new();
        format_datetime(
            Some(2024),
            Some(5),
            Some(17),
            Some(14),
            Some(30),
            None,
            Some("short"),
            Some("medium"),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            &mut out,
            &en(),
        );
        assert!(
            out.contains("24") && (out.contains("PM") || out.contains("14")),
            "got: {out}"
        );
    }
    #[test]
    fn hour12_true() {
        let mut out = String::new();
        format_datetime(
            Some(2024),
            Some(5),
            Some(17),
            Some(13),
            Some(0),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(true),
            None,
            &mut out,
            &en(),
        );
        assert!(out.contains("1") && out.contains("PM"), "got: {out}");
    }
    #[test]
    fn hour12_false() {
        let mut out = String::new();
        format_datetime(
            Some(2024),
            Some(5),
            Some(17),
            Some(13),
            Some(0),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(false),
            None,
            &mut out,
            &en(),
        );
        assert!(out.contains("13"), "got: {out}");
    }

    #[test]
    fn zone_date_only() {
        let mut out = String::new();
        format_datetime(
            Some(2024),
            Some(5),
            Some(17),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some("long"),
            None,
            None,
            &mut out,
            &en(),
        );
        assert!(out.contains("2024") || out.contains("UTC"), "got: {out}");
    }
    #[test]
    fn zone_datetime() {
        let mut out = String::new();
        format_datetime(
            Some(2024),
            Some(5),
            Some(17),
            Some(14),
            Some(30),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some("short"),
            None,
            None,
            &mut out,
            &en(),
        );
        assert!(out.contains("2024") || out.contains("UTC"), "got: {out}");
    }
    #[test]
    fn zone_ind_time() {
        let mut out = String::new();
        format_datetime(
            Some(2024),
            Some(5),
            Some(17),
            Some(10),
            Some(20),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some("Asia/Shanghai"),
            &mut out,
            &en(),
        );
        assert!(out.contains("2024") || out.contains("UTC"), "got: {out}");
    }
    #[test]
    fn zone_ind_no_time() {
        let mut out = String::new();
        format_datetime(
            Some(2024),
            Some(5),
            Some(17),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some("numeric"),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some("UTC"),
            &mut out,
            &en(),
        );
        assert!(out.contains("2024") || out.contains("UTC"), "got: {out}");
    }
    #[test]
    fn zone_ind_wd_time() {
        let mut out = String::new();
        format_datetime(
            Some(2024),
            Some(5),
            Some(17),
            Some(14),
            Some(30),
            None,
            None,
            None,
            Some("long"),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some("America/New_York"),
            &mut out,
            &en(),
        );
        assert!(out.contains("2024") || out.contains("UTC"), "got: {out}");
    }
    #[test]
    fn zone_style_time() {
        let mut out = String::new();
        format_datetime(
            Some(2024),
            Some(5),
            Some(17),
            Some(14),
            Some(30),
            None,
            Some("long"),
            Some("short"),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some("short"),
            None,
            None,
            &mut out,
            &en(),
        );
        assert!(out.contains("2024") || out.contains("UTC"), "got: {out}");
    }

    #[test]
    fn zone_ind_has_zone_time() {
        let mut out = String::new();
        format_datetime(
            Some(2024),
            Some(5),
            Some(17),
            Some(10),
            Some(20),
            None,
            None,
            None,
            None,
            None,
            Some("numeric"),
            None,
            None,
            None,
            None,
            None,
            Some("long"),
            None,
            None,
            &mut out,
            &en(),
        );
        assert!(out.contains("2024") || out.contains("UTC"), "got: {out}");
    }
    #[test]
    fn zone_ind_no_time_tz() {
        let mut out = String::new();
        format_datetime(
            Some(2024),
            Some(5),
            Some(17),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some("numeric"),
            None,
            None,
            None,
            None,
            None,
            Some("short"),
            None,
            Some("Europe/London"),
            &mut out,
            &en(),
        );
        assert!(out.contains("2024") || out.contains("UTC"), "got: {out}");
    }
    #[test]
    fn zone_ind_wd_no_time_tz() {
        let mut out = String::new();
        format_datetime(
            Some(2024),
            Some(5),
            Some(17),
            None,
            None,
            None,
            None,
            None,
            Some("long"),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some("long"),
            None,
            None,
            &mut out,
            &en(),
        );
        assert!(out.contains("2024") || out.contains("UTC"), "got: {out}");
    }
    #[test]
    fn zone_ind_wd_time_tz() {
        let mut out = String::new();
        format_datetime(
            Some(2024),
            Some(5),
            Some(17),
            Some(10),
            Some(20),
            Some(30),
            None,
            None,
            Some("long"),
            None,
            None,
            Some("long"),
            None,
            None,
            None,
            Some("numeric"),
            None,
            None,
            Some("Asia/Tokyo"),
            &mut out,
            &en(),
        );
        assert!(out.contains("2024") || out.contains("UTC"), "got: {out}");
    }
    #[test]
    fn zone_style_tz() {
        let mut out = String::new();
        format_datetime(
            Some(2024),
            Some(5),
            Some(17),
            Some(14),
            Some(30),
            None,
            Some("long"),
            Some("short"),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some("America/Chicago"),
            &mut out,
            &en(),
        );
        assert!(out.contains("2024") || out.contains("UTC"), "got: {out}");
    }

    #[test]
    fn zone_ind_wd_no_time_short() {
        let mut out = String::new();
        format_datetime(
            Some(2024),
            Some(5),
            Some(17),
            None,
            None,
            None,
            None,
            None,
            Some("long"),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some("short"),
            None,
            None,
            &mut out,
            &en(),
        );
        assert!(out.contains("2024") || out.contains("UTC"), "got: {out}");
    }
    #[test]
    fn zone_ind_time_short() {
        let mut out = String::new();
        format_datetime(
            Some(2024),
            Some(5),
            Some(17),
            Some(10),
            Some(20),
            None,
            None,
            None,
            None,
            None,
            Some("numeric"),
            None,
            None,
            None,
            None,
            None,
            Some("short"),
            None,
            None,
            &mut out,
            &en(),
        );
        assert!(out.contains("2024") || out.contains("UTC"), "got: {out}");
    }
    #[test]
    fn zone_ind_wd_time_short() {
        let mut out = String::new();
        format_datetime(
            Some(2024),
            Some(5),
            Some(17),
            Some(10),
            Some(20),
            None,
            None,
            None,
            Some("long"),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some("short"),
            None,
            None,
            &mut out,
            &en(),
        );
        assert!(out.contains("2024") || out.contains("UTC"), "got: {out}");
    }

    #[test]
    fn ind_wd_era_time() {
        let mut out = String::new();
        format_datetime(
            Some(2024),
            Some(5),
            Some(17),
            Some(14),
            Some(30),
            None,
            None,
            None,
            Some("long"),
            Some("long"),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            &mut out,
            &en(),
        );
        assert!(out.contains("2024"), "got: {out}");
    }
    #[test]
    fn ind_wd_era_no_time() {
        let mut out = String::new();
        format_datetime(
            Some(2024),
            Some(5),
            Some(17),
            None,
            None,
            None,
            None,
            None,
            Some("long"),
            Some("long"),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            &mut out,
            &en(),
        );
        assert!(out.contains("2024"), "got: {out}");
    }
    #[test]
    fn ind_era_time() {
        let mut out = String::new();
        format_datetime(
            Some(2024),
            Some(5),
            Some(17),
            Some(14),
            Some(30),
            None,
            None,
            None,
            None,
            Some("long"),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            &mut out,
            &en(),
        );
        assert!(out.contains("2024"), "got: {out}");
    }

    #[test]
    fn z_date_long() {
        let mut out = String::new();
        format_datetime(
            Some(2024),
            Some(5),
            Some(17),
            None,
            None,
            None,
            Some("long"),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some("long"),
            None,
            None,
            &mut out,
            &en(),
        );
        assert!(out.contains("24"), "got:{out}");
    }
    #[test]
    fn z_date_short() {
        let mut out = String::new();
        format_datetime(
            Some(2024),
            Some(5),
            Some(17),
            None,
            None,
            None,
            Some("short"),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some("short"),
            None,
            None,
            &mut out,
            &en(),
        );
        assert!(out.contains("24"), "got:{out}");
    }
    #[test]
    fn z_long_long() {
        let mut out = String::new();
        format_datetime(
            Some(2024),
            Some(5),
            Some(17),
            Some(14),
            Some(30),
            Some(15),
            Some("long"),
            Some("long"),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some("long"),
            None,
            None,
            &mut out,
            &en(),
        );
        assert!(out.contains("24"), "got:{out}");
    }
    #[test]
    fn z_short_short() {
        let mut out = String::new();
        format_datetime(
            Some(2024),
            Some(5),
            Some(17),
            Some(14),
            Some(30),
            None,
            Some("short"),
            Some("short"),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some("short"),
            None,
            None,
            &mut out,
            &en(),
        );
        assert!(out.contains("24"), "got:{out}");
    }
    #[test]
    fn z_short_med() {
        let mut out = String::new();
        format_datetime(
            Some(2024),
            Some(5),
            Some(17),
            Some(14),
            Some(30),
            None,
            Some("short"),
            Some("medium"),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some("short"),
            None,
            None,
            &mut out,
            &en(),
        );
        assert!(out.contains("24"), "got:{out}");
    }

    #[test]
    fn z_wd_era_no_time() {
        let mut out = String::new();
        format_datetime(
            Some(2024),
            Some(5),
            Some(17),
            None,
            None,
            None,
            None,
            None,
            Some("long"),
            Some("long"),
            None,
            None,
            None,
            None,
            None,
            None,
            Some("long"),
            None,
            Some("UTC"),
            &mut out,
            &en(),
        );
        assert!(out.contains("24"), "got:{out}");
    }
    #[test]
    fn z_wd_era_time() {
        let mut out = String::new();
        format_datetime(
            Some(2024),
            Some(5),
            Some(17),
            Some(14),
            Some(30),
            None,
            None,
            None,
            Some("long"),
            Some("long"),
            None,
            None,
            None,
            None,
            None,
            None,
            Some("long"),
            None,
            Some("UTC"),
            &mut out,
            &en(),
        );
        assert!(out.contains("24"), "got:{out}");
    }
    #[test]
    fn z_era_no_wd_time() {
        let mut out = String::new();
        format_datetime(
            Some(2024),
            Some(5),
            Some(17),
            Some(14),
            Some(30),
            None,
            None,
            None,
            None,
            Some("long"),
            None,
            None,
            None,
            None,
            None,
            None,
            Some("long"),
            None,
            Some("UTC"),
            &mut out,
            &en(),
        );
        assert!(out.contains("2024") || out.contains("UTC"), "got:{out}");
    }
    #[test]
    fn z_era_no_wd_no_time() {
        let mut out = String::new();
        format_datetime(
            Some(2024),
            Some(5),
            Some(17),
            None,
            None,
            None,
            None,
            None,
            None,
            Some("long"),
            None,
            None,
            None,
            None,
            None,
            None,
            Some("long"),
            None,
            Some("UTC"),
            &mut out,
            &en(),
        );
        assert!(out.contains("2024") || out.contains("UTC"), "got:{out}");
    }
}

fn format_style_date_only(
    date: Date<Gregorian>,
    ds: &str,
    prefs: &icu_datetime::DateTimeFormatterPreferences,
    out: &mut String,
) {
    let fs = match ds {
        "long" => YMD::long(),
        "short" => YMD::short(),
        _ => YMD::medium(),
    };
    let fmt = FixedCalendarDateTimeFormatter::<Gregorian, _>::try_new(prefs.clone(), fs)
        .expect("compiled_data should always be available");
    let _ = fmt.format(&date).write_to(out);
}

fn format_style_with_time(
    dt: DateTime<Gregorian>,
    ds: &str,
    ts: &str,
    prefs: &icu_datetime::DateTimeFormatterPreferences,
    out: &mut String,
) {
    let (length, tp) = match (ds, ts) {
        ("long", "long") => (Length::Long, TimePrecision::Second),
        ("long", _) => (Length::Long, TimePrecision::Minute),
        ("short", "short") => (Length::Short, TimePrecision::Minute),
        ("short", _) => (Length::Short, TimePrecision::Second),
        _ => (Length::Medium, TimePrecision::Second),
    };
    let mut fs = YMDT::for_length(length);
    fs.time_precision = Some(tp);
    let fmt = DateTimeFormatter::try_new(prefs.clone(), fs)
        .expect("compiled_data should always be available");
    let _ = fmt.format(&dt).write_to(out);
}

fn pick_length(yf: Option<&str>, mf: Option<&str>, df: Option<&str>, wd: Option<&str>) -> Length {
    for &v in &[yf, mf, df, wd] {
        if let Some(s) = v {
            match s {
                "long" => return Length::Long,
                "short" | "narrow" | "2-digit" => return Length::Short,
                _ => {}
            }
        }
    }
    Length::Medium
}

fn pick_time_precision(_hf: Option<&str>, _mf: Option<&str>, sf: Option<&str>) -> TimePrecision {
    if sf.is_some() {
        TimePrecision::Second
    } else {
        TimePrecision::Minute
    }
}

fn resolve_prefs(
    locale: &unic_langid::LanguageIdentifier,
    hour12: Option<bool>,
) -> icu_datetime::DateTimeFormatterPreferences {
    let suffix = match hour12 {
        Some(true) => "-u-hc-h12",
        Some(false) => "-u-hc-h23",
        None => return to_prefs(locale),
    };
    let s = format!("{}{}", locale.to_string(), suffix);
    s.parse::<icu_locale_core::Locale>()
        .ok()
        .map(|l| l.into())
        .unwrap_or_default()
}

fn to_prefs(
    locale: &unic_langid::LanguageIdentifier,
) -> icu_datetime::DateTimeFormatterPreferences {
    let s = locale.to_string();
    s.parse::<icu_locale_core::Locale>()
        .ok()
        .map(|l| l.into())
        .unwrap_or_default()
}
