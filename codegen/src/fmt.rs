use std::collections::BTreeMap;
use std::fmt::Write;

use crate::ast::*;
use crate::plural::plural_rule_or_fallback;
use crate::util::{escape_str, sanitize};
use crate::{BuiltInArgType, BuiltInFuncDef};

pub fn generate_one_function(
    name: &str,
    elements: &[Element],
    params: &BTreeMap<String, ParamType>,
    locale: &str,
    builtins: &BTreeMap<String, BuiltInFuncDef>,
) -> String {
    let mut code = String::new();
    let decl = gen_fn_decl(name, params, false, builtins);

    if is_pure_text(elements) {
        let text: String = elements
            .iter()
            .filter_map(|e| {
                if let Element::Text(t) = e {
                    Some(t.as_str())
                } else {
                    None
                }
            })
            .collect();
        writeln!(code, "#[inline]").unwrap();
        writeln!(code, "pub fn {} {{ \"{}\" }}", decl, escape_str(&text)).unwrap();
    } else if has_select(elements) {
        writeln!(code, "#[inline]").unwrap();
        writeln!(code, "pub fn {} {{", decl).unwrap();
        writeln!(
            code,
            "{}",
            gen_select_body(elements, params, locale, builtins)
        )
        .unwrap();
        writeln!(code, "}}").unwrap();
    } else {
        writeln!(code, "#[inline]").unwrap();
        writeln!(code, "pub fn {} {{", decl).unwrap();
        writeln!(code, "{}", gen_push_body(elements, params, builtins)).unwrap();
        writeln!(code, "}}").unwrap();
    }
    code
}

pub fn gen_fn_decl(
    name: &str,
    params: &BTreeMap<String, ParamType>,
    with_self: bool,
    _builtins: &BTreeMap<String, BuiltInFuncDef>,
) -> String {
    let mut out = String::new();
    let safe_name = sanitize(name);
    let has_str = params.values().any(|t| *t == ParamType::Str);
    write!(out, "{}", safe_name).unwrap();
    if has_str {
        write!(out, "<'a>").unwrap();
    }
    write!(out, "(").unwrap();
    let mut first = true;
    if with_self {
        write!(out, "&self").unwrap();
        first = false;
    }
    for (pname, ptype) in params {
        if !first {
            write!(out, ", ").unwrap();
        }
        first = false;
        let safe_pname = sanitize(pname);
        write!(out, "{}: ", safe_pname).unwrap();
        match ptype {
            ParamType::Str => write!(out, "impl Into<ftl_core::FluentArg<'a>>").unwrap(),
            ParamType::Num => write!(out, "impl Into<FluentNum>").unwrap(),
            ParamType::Builtin(ty) => write!(out, "{}", ty).unwrap(),
        }
    }
    write!(out, ")").unwrap();
    if params.is_empty() {
        write!(out, " -> &'static str").unwrap();
    } else {
        write!(out, " -> String").unwrap();
    }
    out
}

fn has_select(elements: &[Element]) -> bool {
    elements.iter().any(|e| matches!(e, Element::Select { .. }))
}

fn is_pure_text(elements: &[Element]) -> bool {
    elements.iter().all(|e| matches!(e, Element::Text(_)))
}

fn capacity_expr(elements: &[Element], params: &BTreeMap<String, ParamType>) -> String {
    let mut parts: Vec<String> = Vec::new();
    let mut acc: usize = 0;
    for e in elements {
        match e {
            Element::Text(t) => acc += t.len(),
            Element::VarRef(name) => {
                if acc > 0 {
                    parts.push(acc.to_string());
                    acc = 0;
                }
                parts.push(match params.get(name).unwrap_or(&ParamType::Str) {
                    ParamType::Num => "32".to_string(),
                    ParamType::Str => "32".to_string(),
                    ParamType::Builtin(_) => "32".to_string(),
                });
            }
            Element::BuiltInCall { .. } => {
                if acc > 0 {
                    parts.push(acc.to_string());
                    acc = 0;
                }
                parts.push("32".to_string());
            }
            Element::Select { .. } => unreachable!(),
            Element::MessageRef(_)
            | Element::TermRef { .. }
            | Element::AttributeRef { .. }
            | Element::TermAttrSelect { .. } => unreachable!(),
        }
    }
    if acc > 0 {
        parts.push(acc.to_string());
    }
    if parts.is_empty() {
        return "0".to_string();
    }
    parts.join(" + ")
}

fn emit_push_statements(
    elements: &[Element],
    params: &BTreeMap<String, ParamType>,
    indent: &str,
    builtins: &BTreeMap<String, BuiltInFuncDef>,
    code: &mut String,
) {
    for e in elements {
        match e {
            Element::Text(t) => {
                writeln!(code, "{}s.push_str(\"{}\");", indent, escape_str(t)).unwrap();
            }
            Element::VarRef(name) => {
                let safe_name = sanitize(name);
                match params.get(name).unwrap_or(&ParamType::Str) {
                    ParamType::Num => {
                        writeln!(
                            code,
                            "{}write!(&mut s, \"{{}}\", {}).unwrap();",
                            indent, safe_name
                        )
                        .unwrap();
                    }
                    ParamType::Str => {
                        writeln!(
                            code,
                            "{}{}.into().write_localized(&mut s);",
                            indent, safe_name
                        )
                        .unwrap();
                    }
                    ParamType::Builtin(_) => {
                        writeln!(code, "{}{}.write_to(&mut s);", indent, safe_name).unwrap();
                    }
                }
            }
            Element::BuiltInCall {
                func_name,
                var_name,
                named_args,
                ..
            } if func_name == "NUMBER" => {
                let safe_var = sanitize(var_name);
                let v = |name: &str| -> String {
                    match named_args.get(name) {
                        Some(raw) => format!("Some({}i64)", raw),
                        None => "None".to_string(),
                    }
                };
                let v_bool = |name: &str| -> String {
                    match named_args.get(name) {
                        Some(raw) if raw.eq_ignore_ascii_case("true") || raw == "1" => {
                            "Some(true)".to_string()
                        }
                        Some(_) => "Some(false)".to_string(),
                        None => "None".to_string(),
                    }
                };
                let v_str = |name: &str| -> String {
                    match named_args.get(name) {
                        Some(raw) => format!("Some(\"{}\")", raw),
                        None => "None".to_string(),
                    }
                };
                writeln!(
                    code,
                    "{}ftl_core::number::format(*{}, {}, {}, {}, {}, {}, {}, {}, {}, {}, &mut s, &ftl_core::locale());",
                    indent,
                    safe_var,
                    v("minimumFractionDigits"),
                    v("maximumFractionDigits"),
                    v("minimumSignificantDigits"),
                    v("maximumSignificantDigits"),
                    v("minimumIntegerDigits"),
                    v_bool("useGrouping"),
                    v_str("style"),
                    v_str("currency"),
                    v_str("currencyDisplay"),
                )
                .unwrap();
            }
            Element::BuiltInCall {
                func_name,
                named_args,
                ..
            } if func_name == "DATETIME" => {
                let v_i64 = |name: &str| -> String {
                    match named_args.get(name) {
                        Some(raw) => format!("Some({}i64)", raw),
                        None => "None".to_string(),
                    }
                };
                let v_bool = |name: &str| -> String {
                    match named_args.get(name) {
                        Some(raw) if raw.eq_ignore_ascii_case("true") || raw == "1" => {
                            "Some(true)".to_string()
                        }
                        Some(_) => "Some(false)".to_string(),
                        None => "None".to_string(),
                    }
                };
                let v_str = |name: &str| -> String {
                    match named_args.get(name) {
                        Some(raw) => format!("Some(\"{}\")", raw),
                        None => "None".to_string(),
                    }
                };
                writeln!(
                    code,
                    "{}ftl_core::datetime::format({}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, &mut s, &ftl_core::locale());",
                    indent,
                    v_i64("year"),
                    v_i64("month"),
                    v_i64("day"),
                    v_i64("hour"),
                    v_i64("minute"),
                    v_i64("second"),
                    v_str("dateStyle"),
                    v_str("timeStyle"),
                    v_str("weekday"),
                    v_str("era"),
                    v_str("yearFormat"),
                    v_str("monthFormat"),
                    v_str("dayFormat"),
                    v_str("hourFormat"),
                    v_str("minuteFormat"),
                    v_str("secondFormat"),
                    v_str("timeZoneName"),
                    v_bool("hour12"),
                    v_str("timeZone"),
                )
                .unwrap();
            }
            Element::BuiltInCall {
                func_name,
                var_name,
                named_args,
                ..
            } => {
                let safe_var = sanitize(var_name);
                let def = builtins
                    .get(func_name.as_str())
                    .unwrap_or_else(|| panic!("Built-in function '{}' not registered", func_name));
                write!(code, "{}{}", indent, safe_var).unwrap();
                for arg_def in def.named_args.iter() {
                    if let Some(raw_value) = named_args.get(&arg_def.ftl_name) {
                        let rust_value = format_builtin_arg(raw_value, &arg_def.arg_type);
                        write!(code, ".{}({})", arg_def.rust_name, rust_value).unwrap();
                    }
                }
                writeln!(code, ".write_to(&mut s);").unwrap();
            }
            Element::Select { .. } => unreachable!(),
            Element::MessageRef(_)
            | Element::TermRef { .. }
            | Element::AttributeRef { .. }
            | Element::TermAttrSelect { .. } => unreachable!(),
        }
    }
}

fn gen_push_body(
    elements: &[Element],
    params: &BTreeMap<String, ParamType>,
    builtins: &BTreeMap<String, BuiltInFuncDef>,
) -> String {
    let mut code = String::new();
    writeln!(code, "    let cap = {};", capacity_expr(elements, params)).unwrap();
    emit_num_convert(params, "    ", &mut code);
    writeln!(code, "    let mut s = String::with_capacity(cap);").unwrap();
    emit_push_statements(elements, params, "    ", builtins, &mut code);
    writeln!(code, "    s").unwrap();
    code
}

fn gen_select_body(
    elements: &[Element],
    params: &BTreeMap<String, ParamType>,
    locale: &str,
    builtins: &BTreeMap<String, BuiltInFuncDef>,
) -> String {
    let s = elements
        .iter()
        .find_map(|e| {
            if let Element::Select { selector, variants } = e {
                Some((selector.clone(), variants))
            } else {
                None
            }
        })
        .expect("gen_select_body called without Select");
    let (selector, variants) = s;
    let selector_type = params.get(&selector).unwrap_or(&ParamType::Num);
    let safe_selector = sanitize(&selector);

    let mut code = String::new();

    if let ParamType::Builtin(_) = selector_type {
        panic!(
            "Cannot use '{}' (built-in type) as a select selector",
            selector
        );
    }

    if selector_type == &ParamType::Str {
        writeln!(
            code,
            "    let __sel: String = match {}.into() {{",
            safe_selector
        )
        .unwrap();
        writeln!(code, "        FluentArg::Str(s) => s.into_owned(),").unwrap();
        writeln!(code, "        _ => return String::new(),").unwrap();
        writeln!(code, "    }};").unwrap();
        writeln!(code, "    match __sel.as_str() {{").unwrap();
        for v in variants.iter().filter(|v| !v.default) {
            writeln!(
                code,
                "        {} => {},",
                variant_arm_pattern(&v.key, selector_type, locale, v.default),
                variant_arm_body(&v.elements, params, builtins)
            )
            .unwrap();
        }
        for v in variants.iter().filter(|v| v.default) {
            writeln!(
                code,
                "        {} => {},",
                variant_arm_pattern(&v.key, selector_type, locale, v.default),
                variant_arm_body(&v.elements, params, builtins)
            )
            .unwrap();
        }
        writeln!(code, "    }}").unwrap();
    } else {
        writeln!(
            code,
            "    let {}: FluentNum = {}.into();",
            safe_selector, safe_selector
        )
        .unwrap();
        writeln!(code, "    match *{} {{", safe_selector).unwrap();
        for v in variants.iter().filter(|v| !v.default) {
            writeln!(
                code,
                "        {} => {},",
                variant_arm_pattern_num(&v.key, locale),
                variant_arm_body(&v.elements, params, builtins)
            )
            .unwrap();
        }
        for v in variants.iter().filter(|v| v.default) {
            writeln!(
                code,
                "        _ => {},",
                variant_arm_body(&v.elements, params, builtins)
            )
            .unwrap();
        }
        writeln!(code, "    }}").unwrap();
    }
    code
}

fn variant_arm_body(
    elements: &[Element],
    params: &BTreeMap<String, ParamType>,
    builtins: &BTreeMap<String, BuiltInFuncDef>,
) -> String {
    if is_pure_text(elements) {
        let text: String = elements
            .iter()
            .filter_map(|e| {
                if let Element::Text(t) = e {
                    Some(t.as_str())
                } else {
                    None
                }
            })
            .collect();
        return format!("\"{}\".to_string()", escape_str(&text));
    }
    let mut code = String::new();
    writeln!(code, "{{").unwrap();
    writeln!(
        code,
        "            let cap = {};",
        capacity_expr(elements, params)
    )
    .unwrap();
    writeln!(code, "            let mut s = String::with_capacity(cap);").unwrap();
    emit_push_statements(elements, params, "            ", builtins, &mut code);
    writeln!(code, "            s").unwrap();
    write!(code, "        }}").unwrap();
    code
}

fn variant_arm_pattern(
    key: &KeyType,
    selector_type: &ParamType,
    locale: &str,
    default: bool,
) -> String {
    if default {
        return "_".to_string();
    }
    match (key, selector_type) {
        (KeyType::Num(val), _) => val.clone(),
        (KeyType::Ident(cat), ParamType::Num) => plural_rule_or_fallback(locale, cat)
            .map(|rule| match rule {
                "n == 1" => "1".to_string(),
                "n == 0" => "0".to_string(),
                "n == 2" => "2".to_string(),
                r => format!("n if {}", r),
            })
            .unwrap_or_else(|| "n if false".to_string()),
        (KeyType::Ident(cat), ParamType::Str) => format!("\"{}\"", escape_str(cat)),
        (_, ParamType::Builtin(_)) => {
            panic!("Built-in type cannot be used in select variant pattern");
        }
    }
}

fn format_builtin_arg(raw: &str, arg_type: &BuiltInArgType) -> String {
    match arg_type {
        BuiltInArgType::String => format!("\"{}\".to_string()", raw),
        BuiltInArgType::Int => format!("{}i64", raw),
        BuiltInArgType::Float => format!("{}f64", raw),
        BuiltInArgType::Bool => {
            if raw.eq_ignore_ascii_case("true") || raw == "1" {
                "true".to_string()
            } else {
                "false".to_string()
            }
        }
    }
}

/// Generate `.into()` shadowing lines for numeric params.
fn emit_num_convert(params: &BTreeMap<String, ParamType>, indent: &str, code: &mut String) {
    for (pname, ptype) in params {
        if *ptype == ParamType::Num {
            let safe = sanitize(pname);
            writeln!(code, "{}let {}: FluentNum = {}.into();", indent, safe, safe).unwrap();
        }
    }
}

/// Generate a match arm pattern for a numeric variant.
fn variant_arm_pattern_num(key: &KeyType, locale: &str) -> String {
    match key {
        KeyType::Num(val) => format!("{}.0", val),
        KeyType::Ident(cat) => match plural_rule_or_fallback(locale, cat) {
            Some("n == 0") => "0.0".to_string(),
            Some("n == 1") => "1.0".to_string(),
            Some("n == 2") => "2.0".to_string(),
            Some(rule) => {
                // CLDR rules use integer n; convert f64 match var to i64
                let guarded = rule
                    .replace("n !=", "(n.trunc() as i64) !=")
                    .replace("n ==", "(n.trunc() as i64) ==")
                    .replace("n %", "(n.trunc() as i64) %");
                format!("n if {}", guarded)
            }
            None => "false".to_string(),
        },
    }
}

#[cfg(test)]
#[cfg_attr(coverage, coverage(off))]
mod tests {
    use super::*;
    use crate::ast::{Element, KeyType, ParamType, Variant};
    use crate::{BuiltInArgType, BuiltInFuncDef, BuiltInNamedArg};
    use std::collections::BTreeMap;

    fn no_builtins() -> BTreeMap<String, BuiltInFuncDef> {
        BTreeMap::new()
    }

    #[test]
    fn detect_pure_text() {
        assert!(is_pure_text(&[Element::Text("hello".into())]));
        assert!(!is_pure_text(&[
            Element::Text("a".into()),
            Element::VarRef("x".into())
        ]));
    }

    #[test]
    fn detect_has_select() {
        assert!(!has_select(&[Element::Text("a".into())]));
        assert!(!has_select(&[Element::VarRef("x".into())]));
    }

    #[test]
    fn fn_decl_no_params() {
        let params = BTreeMap::new();
        let b = no_builtins();
        assert_eq!(
            gen_fn_decl("settings", &params, false, &b),
            "settings() -> &'static str"
        );
    }

    #[test]
    fn fn_decl_with_self() {
        let mut params = BTreeMap::new();
        params.insert("name".into(), ParamType::Str);
        let b = no_builtins();
        assert_eq!(
            gen_fn_decl("hello", &params, true, &b),
            "hello<'a>(&self, name: impl Into<ftl_core::FluentArg<'a>>) -> String"
        );
    }

    #[test]
    fn fn_decl_num_param() {
        let mut params = BTreeMap::new();
        params.insert("count".into(), ParamType::Num);
        let b = no_builtins();
        assert_eq!(
            gen_fn_decl("files", &params, false, &b),
            "files(count: impl Into<FluentNum>) -> String"
        );
    }

    #[test]
    fn capacity_expr_cases() {
        let params = BTreeMap::new();
        let elems = [Element::Text("Hello".into())];
        assert_eq!(capacity_expr(&elems, &params), "5");

        let elems: [Element; 0] = [];
        assert_eq!(capacity_expr(&elems, &params), "0");

        let mut params2 = BTreeMap::new();
        params2.insert("n".into(), ParamType::Num);
        let elems = [Element::VarRef("n".into())];
        assert_eq!(capacity_expr(&elems, &params2), "32");

        let mut params3 = BTreeMap::new();
        params3.insert("name".into(), ParamType::Str);
        let elems = [
            Element::Text("Hello, ".into()),
            Element::VarRef("name".into()),
            Element::Text("!".into()),
        ];
        assert_eq!(capacity_expr(&elems, &params3), "7 + 32 + 1");

        let mut params4 = BTreeMap::new();
        params4.insert("a".into(), ParamType::Str);
        params4.insert("b".into(), ParamType::Num);
        let elems = [
            Element::Text("x".into()),
            Element::VarRef("a".into()),
            Element::Text("yz".into()),
            Element::VarRef("b".into()),
        ];
        assert_eq!(capacity_expr(&elems, &params4), "1 + 32 + 2 + 32");
    }

    #[test]
    fn arm_pattern_cases() {
        assert_eq!(
            variant_arm_pattern(&KeyType::Ident("other".into()), &ParamType::Num, "en", true),
            "_"
        );
        assert_eq!(
            variant_arm_pattern(&KeyType::Num("42".into()), &ParamType::Num, "en", false),
            "42"
        );
        assert_eq!(
            variant_arm_pattern(&KeyType::Ident("one".into()), &ParamType::Num, "en", false),
            "1"
        );
        assert_eq!(
            variant_arm_pattern(&KeyType::Ident("zero".into()), &ParamType::Num, "en", false),
            "0"
        );
        assert_eq!(
            variant_arm_pattern(&KeyType::Ident("two".into()), &ParamType::Num, "en", false),
            "2"
        );
        assert_eq!(
            variant_arm_pattern(&KeyType::Ident("few".into()), &ParamType::Num, "en", false),
            "n if false"
        );
        let pat = variant_arm_pattern(&KeyType::Ident("one".into()), &ParamType::Num, "ru", false);
        assert!(pat.contains("n % 10 == 1"));
        assert_eq!(
            variant_arm_pattern(&KeyType::Ident("male".into()), &ParamType::Str, "en", false),
            "\"male\""
        );
    }

    #[test]
    fn generate_function_cases() {
        let params = BTreeMap::new();
        let elems = [Element::Text("Settings".into())];
        let code = generate_one_function("settings", &elems, &params, "en", &no_builtins());
        assert_eq!(
            code.trim(),
            "#[inline]\npub fn settings() -> &'static str { \"Settings\" }"
        );

        let params = BTreeMap::new();
        let elems = [Element::Text("he\"llo\nworld".into())];
        let code = generate_one_function("test", &elems, &params, "en", &no_builtins());
        assert!(code.contains("he\\\"llo\\nworld"));

        let mut params = BTreeMap::new();
        params.insert("name".into(), ParamType::Str);
        let elems = [
            Element::Text("Hello, ".into()),
            Element::VarRef("name".into()),
            Element::Text("!".into()),
        ];
        let code = generate_one_function("hello", &elems, &params, "en", &no_builtins());
        assert!(
            code.contains("pub fn hello<'a>(name: impl Into<ftl_core::FluentArg<'a>>) -> String {")
        );
        assert!(code.contains("let cap = 7 + 32 + 1;"));
        assert!(code.contains("name.into().write_localized(&mut s);"));

        let mut params = BTreeMap::new();
        params.insert("count".into(), ParamType::Num);
        let elems = [
            Element::Text("count: ".into()),
            Element::VarRef("count".into()),
        ];
        let code = generate_one_function("show_count", &elems, &params, "en", &no_builtins());
        assert!(code.contains("show_count"));
        assert!(code.contains("count: impl Into<FluentNum>"));
        assert!(code.contains("write!(&mut s, \"{}\", count).unwrap();"));

        let mut params = BTreeMap::new();
        params.insert("count".into(), ParamType::Num);
        let elems = [Element::Select {
            selector: "count".into(),
            variants: vec![
                Variant {
                    key: KeyType::Ident("one".into()),
                    elements: vec![Element::Text("1 file".into())],
                    default: false,
                },
                Variant {
                    key: KeyType::Ident("other".into()),
                    elements: vec![
                        Element::VarRef("count".into()),
                        Element::Text(" files".into()),
                    ],
                    default: true,
                },
            ],
        }];
        let code = generate_one_function("files", &elems, &params, "en", &no_builtins());
        assert!(code.contains("pub fn files(count: impl Into<FluentNum>) -> String {"));
        assert!(code.contains("count: FluentNum = count.into()"));
        assert!(code.contains("1.0 =>"));
    }

    #[test]
    fn format_builtin_arg_string() {
        assert_eq!(
            format_builtin_arg("hello", &BuiltInArgType::String),
            "\"hello\".to_string()"
        );
    }

    #[test]
    fn format_builtin_arg_int() {
        assert_eq!(format_builtin_arg("42", &BuiltInArgType::Int), "42i64");
    }

    #[test]
    fn format_builtin_arg_float() {
        assert_eq!(
            format_builtin_arg("3.14", &BuiltInArgType::Float),
            "3.14f64"
        );
    }

    #[test]
    fn format_builtin_arg_bool_true() {
        assert_eq!(format_builtin_arg("true", &BuiltInArgType::Bool), "true");
    }

    #[test]
    fn format_builtin_arg_bool_false() {
        assert_eq!(format_builtin_arg("false", &BuiltInArgType::Bool), "false");
    }

    #[test]
    fn format_builtin_arg_bool_one() {
        assert_eq!(format_builtin_arg("1", &BuiltInArgType::Bool), "true");
    }

    #[test]
    fn emit_num_convert_generates_into_lines() {
        let mut params = BTreeMap::new();
        params.insert("count".into(), ParamType::Num);
        params.insert("name".into(), ParamType::Str);
        let mut code = String::new();
        emit_num_convert(&params, "    ", &mut code);
        assert!(code.contains("let count: FluentNum = count.into();"));
        assert!(!code.contains("name"));
    }

    #[test]
    fn custom_builtin_call_emit_statements() {
        let mut builtins = no_builtins();
        builtins.insert(
            "HTML".to_string(),
            BuiltInFuncDef {
                name: "HTML".to_string(),
                ty_name: "Html".to_string(),
                named_args: vec![
                    BuiltInNamedArg {
                        ftl_name: "class".to_string(),
                        rust_name: "class".to_string(),
                        arg_type: BuiltInArgType::String,
                    },
                    BuiltInNamedArg {
                        ftl_name: "count".to_string(),
                        rust_name: "count".to_string(),
                        arg_type: BuiltInArgType::Int,
                    },
                    BuiltInNamedArg {
                        ftl_name: "ratio".to_string(),
                        rust_name: "ratio".to_string(),
                        arg_type: BuiltInArgType::Float,
                    },
                    BuiltInNamedArg {
                        ftl_name: "enabled".to_string(),
                        rust_name: "enabled".to_string(),
                        arg_type: BuiltInArgType::Bool,
                    },
                ],
                write_to_body: None,
            },
        );

        let mut params = BTreeMap::new();
        params.insert("input".into(), ParamType::Builtin("Html".to_string()));

        let elems = [Element::BuiltInCall {
            func_name: "HTML".to_string(),
            var_name: "input".to_string(),
            ty_name: "Html".to_string(),
            named_args: [
                ("class".to_string(), "btn".to_string()),
                ("count".to_string(), "3".to_string()),
                ("ratio".to_string(), "1.5".to_string()),
                ("enabled".to_string(), "true".to_string()),
            ]
            .into(),
        }];

        let mut code = String::new();
        emit_push_statements(&elems, &params, "    ", &builtins, &mut code);
        assert!(code.contains("input.class"));
        assert!(code.contains("\"btn\".to_string()"));
        assert!(code.contains(".count(3i64)"));
        assert!(code.contains(".ratio(1.5f64)"));
        assert!(code.contains(".enabled(true)"));
        assert!(code.contains(".write_to(&mut s);"));
    }

    #[test]
    fn datetime_builtin_call_emit_statements() {
        let params = BTreeMap::new();
        let builtins = no_builtins();

        let elems = [Element::BuiltInCall {
            func_name: "DATETIME".to_string(),
            var_name: "date".to_string(),
            ty_name: "DateTime".to_string(),
            named_args: [
                ("year".to_string(), "2024".to_string()),
                ("month".to_string(), "5".to_string()),
                ("day".to_string(), "15".to_string()),
                ("hour12".to_string(), "true".to_string()),
                ("timeZone".to_string(), "UTC".to_string()),
            ]
            .into(),
        }];

        let mut code = String::new();
        emit_push_statements(&elems, &params, "    ", &builtins, &mut code);
        assert!(code.contains("ftl_core::datetime::format"));
        assert!(code.contains("Some(2024i64)")); // year
        assert!(code.contains("Some(5i64)")); // month
        assert!(code.contains("Some(15i64)")); // day
        assert!(code.contains("Some(true)")); // hour12
        assert!(code.contains("Some(\"UTC\")")); // timeZone
        assert!(code.contains("None")); // unset optional args
    }

    #[test]
    fn variant_arm_pattern_num_all_cases() {
        assert_eq!(
            variant_arm_pattern_num(&KeyType::Num("7".into()), "en"),
            "7.0"
        );
        assert_eq!(
            variant_arm_pattern_num(&KeyType::Ident("zero".into()), "en"),
            "0.0"
        );
        assert_eq!(
            variant_arm_pattern_num(&KeyType::Ident("one".into()), "en"),
            "1.0"
        );
        assert_eq!(
            variant_arm_pattern_num(&KeyType::Ident("two".into()), "en"),
            "2.0"
        );
        assert_eq!(
            variant_arm_pattern_num(&KeyType::Ident("few".into()), "en"),
            "false"
        );
        let pat = variant_arm_pattern_num(&KeyType::Ident("one".into()), "ru");
        assert!(pat.contains("(n.trunc() as i64) % 10 == 1"));
    }

    #[test]
    fn generate_string_selector() {
        let mut params = BTreeMap::new();
        params.insert("gender".into(), ParamType::Str);
        let elems = [Element::Select {
            selector: "gender".into(),
            variants: vec![
                Variant {
                    key: KeyType::Ident("male".into()),
                    elements: vec![Element::Text("sir".into())],
                    default: false,
                },
                Variant {
                    key: KeyType::Ident("other".into()),
                    elements: vec![Element::Text("other".into())],
                    default: true,
                },
            ],
        }];
        let code = generate_one_function("greet", &elems, &params, "en", &no_builtins());
        assert!(code.contains("match __sel.as_str()"));
        assert!(code.contains("\"male\" => \"sir\".to_string()"));
        assert!(code.contains("_ => \"other\".to_string()"));
    }
}
