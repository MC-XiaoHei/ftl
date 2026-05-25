use std::collections::BTreeMap;
use std::fmt::Write;

use crate::ast::*;
use crate::plural::plural_rule_or_fallback;
use crate::util::{escape_str, sanitize};

pub fn generate_one_function(
    name: &str,
    elements: &[Element],
    params: &BTreeMap<String, ParamType>,
    locale: &str,
) -> String {
    let mut code = String::new();
    let decl = gen_fn_decl(name, params, false);

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
        writeln!(code, "pub fn {} {{ \"{}\" }}", decl, escape_str(&text)).unwrap();
    } else if has_select(elements) {
        writeln!(code, "pub fn {} {{", decl).unwrap();
        writeln!(code, "{}", gen_select_body(elements, params, locale)).unwrap();
        writeln!(code, "}}").unwrap();
    } else {
        writeln!(code, "pub fn {} {{", decl).unwrap();
        writeln!(code, "{}", gen_push_body(elements, params)).unwrap();
        writeln!(code, "}}").unwrap();
    }
    code
}

pub fn gen_fn_decl(name: &str, params: &BTreeMap<String, ParamType>, with_self: bool) -> String {
    let mut out = String::new();
    let safe_name = sanitize(name);
    write!(out, "{}(", safe_name).unwrap();
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
        write!(
            out,
            "{}: {}",
            safe_pname,
            match ptype {
                ParamType::Str => "&str",
                ParamType::Num => "impl Into<FluentNum>",
            }
        )
        .unwrap();
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
                let safe_name = sanitize(name);
                parts.push(match params.get(name).unwrap_or(&ParamType::Str) {
                    ParamType::Num => "32".to_string(),
                    ParamType::Str => format!("{}.len()", safe_name),
                });
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
                        writeln!(code, "{}s.push_str({});", indent, safe_name).unwrap();
                    }
                }
            }
            Element::Select { .. } => unreachable!(),
            Element::MessageRef(_)
            | Element::TermRef { .. }
            | Element::AttributeRef { .. }
            | Element::TermAttrSelect { .. } => unreachable!(),
        }
    }
}

fn gen_push_body(elements: &[Element], params: &BTreeMap<String, ParamType>) -> String {
    let mut code = String::new();
    writeln!(code, "    let cap = {};", capacity_expr(elements, params)).unwrap();
    emit_num_convert(params, "    ", &mut code);
    writeln!(code, "    let mut s = String::with_capacity(cap);").unwrap();
    emit_push_statements(elements, params, "    ", &mut code);
    writeln!(code, "    s").unwrap();
    code
}

fn gen_select_body(
    elements: &[Element],
    params: &BTreeMap<String, ParamType>,
    locale: &str,
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

    if selector_type == &ParamType::Str {
        writeln!(code, "    match {} {{", safe_selector).unwrap();
        for v in variants.iter().filter(|v| !v.default) {
            writeln!(
                code,
                "        {} => {},",
                variant_arm_pattern(&v.key, selector_type, locale, v.default),
                variant_arm_body(&v.elements, params)
            )
            .unwrap();
        }
        for v in variants.iter().filter(|v| v.default) {
            writeln!(
                code,
                "        {} => {},",
                variant_arm_pattern(&v.key, selector_type, locale, v.default),
                variant_arm_body(&v.elements, params)
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
        let mut first_arm = true;
        for v in variants.iter().filter(|v| !v.default) {
            let cond = variant_to_cond(&safe_selector, &v.key, locale);
            if first_arm {
                writeln!(code, "    if {} {{", cond).unwrap();
                first_arm = false;
            } else {
                writeln!(code, "    }} else if {} {{", cond).unwrap();
            }
            writeln!(
                code,
                "        return {};",
                variant_arm_body(&v.elements, params)
            )
            .unwrap();
        }
        for v in variants.iter().filter(|v| v.default) {
            writeln!(code, "    }} else {{").unwrap();
            writeln!(
                code,
                "        return {};",
                variant_arm_body(&v.elements, params)
            )
            .unwrap();
        }
        writeln!(code, "    }}").unwrap();
    }
    code
}

fn variant_arm_body(elements: &[Element], params: &BTreeMap<String, ParamType>) -> String {
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
    emit_push_statements(elements, params, "            ", &mut code);
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
    }
}

/// Generate an if-condition for a numeric variant using eq_int or CLDR operands.
fn variant_to_cond(selector: &str, key: &KeyType, locale: &str) -> String {
    match key {
        KeyType::Num(val) => format!("{}.eq_int({})", selector, val),
        KeyType::Ident(cat) => match plural_rule_or_fallback(locale, cat) {
            Some("n == 0") => format!("{}.eq_int(0)", selector),
            Some("n == 1") => format!("{}.eq_int(1)", selector),
            Some("n == 2") => format!("{}.eq_int(2)", selector),
            Some(rule) => {
                // Replace `n` (standalone var) with `selector.operands().i`
                let expr = rule.replace('n', &format!("{}.operands().i", selector));
                expr
            }
            None => "false".to_string(),
        },
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

#[cfg(test)]
#[cfg_attr(coverage, coverage(off))]
mod tests {
    use super::*;
    use crate::ast::{Element, KeyType, ParamType, Variant};
    use std::collections::BTreeMap;

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
        assert_eq!(
            gen_fn_decl("settings", &params, false),
            "settings() -> &'static str"
        );
    }

    #[test]
    fn fn_decl_with_self() {
        let mut params = BTreeMap::new();
        params.insert("name".into(), ParamType::Str);
        assert_eq!(
            gen_fn_decl("hello", &params, true),
            "hello(&self, name: &str) -> String"
        );
    }

    #[test]
    fn fn_decl_num_param() {
        let mut params = BTreeMap::new();
        params.insert("count".into(), ParamType::Num);
        assert_eq!(
            gen_fn_decl("files", &params, false),
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
        assert_eq!(capacity_expr(&elems, &params3), "7 + name.len() + 1");

        let mut params4 = BTreeMap::new();
        params4.insert("a".into(), ParamType::Str);
        params4.insert("b".into(), ParamType::Num);
        let elems = [
            Element::Text("x".into()),
            Element::VarRef("a".into()),
            Element::Text("yz".into()),
            Element::VarRef("b".into()),
        ];
        assert_eq!(capacity_expr(&elems, &params4), "1 + a.len() + 2 + 32");
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
        let code = generate_one_function("settings", &elems, &params, "en");
        assert_eq!(
            code.trim(),
            "pub fn settings() -> &'static str { \"Settings\" }"
        );

        let params = BTreeMap::new();
        let elems = [Element::Text("he\"llo\nworld".into())];
        let code = generate_one_function("test", &elems, &params, "en");
        assert!(code.contains("he\\\"llo\\nworld"));

        let mut params = BTreeMap::new();
        params.insert("name".into(), ParamType::Str);
        let elems = [
            Element::Text("Hello, ".into()),
            Element::VarRef("name".into()),
            Element::Text("!".into()),
        ];
        let code = generate_one_function("hello", &elems, &params, "en");
        assert!(code.starts_with("pub fn hello(name: &str) -> String {"));
        assert!(code.contains("let cap = 7 + name.len() + 1;"));
        assert!(code.contains("s.push_str(name);"));

        let mut params = BTreeMap::new();
        params.insert("count".into(), ParamType::Num);
        let elems = [
            Element::Text("count: ".into()),
            Element::VarRef("count".into()),
        ];
        let code = generate_one_function("show_count", &elems, &params, "en");
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
        let code = generate_one_function("files", &elems, &params, "en");
        assert!(code.starts_with("pub fn files(count: impl Into<FluentNum>) -> String {"));
        assert!(code.contains("count: FluentNum = count.into()"));
        assert!(code.contains("count.eq_int(1)"));
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
        let code = generate_one_function("greet", &elems, &params, "en");
        assert!(code.contains("match gender"));
        assert!(code.contains("\"male\" => \"sir\".to_string()"));
        assert!(code.contains("_ => \"other\".to_string()"));
    }
}
