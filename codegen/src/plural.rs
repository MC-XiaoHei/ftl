use std::collections::HashMap;
use std::sync::OnceLock;

type RuleCache = HashMap<String, Vec<(String, String)>>;

const CLDR_RULE_PREFIX: &str = "pluralRule-count-";
const OTHER: &str = "other";

const OR_SEP: &str = " or ";
const AND_SEP: &str = " and ";
const RANGE_SEP: &str = "..";

const FALLBACK_ZERO: &str = "n == 0";
const FALLBACK_ONE: &str = "n == 1";
const FALLBACK_TWO: &str = "n == 2";

fn load_cldr() -> &'static RuleCache {
    static RULES: OnceLock<RuleCache> = OnceLock::new();
    RULES.get_or_init(|| {
        let root: serde_json::Value = serde_json::from_str(include_str!(
            "../../cldr/cldr-core/supplemental/plurals.json"
        ))
        .expect("Invalid plurals.json");
        let mut rules = RuleCache::new();
        let Some(table) = root["supplemental"]["plurals-type-cardinal"].as_object() else {
            return rules;
        };
        for (tag, entries) in table {
            let Some(categories) = entries.as_object() else {
                continue;
            };
            let mut out = Vec::new();
            for (key, val) in categories {
                let Some(cat) = key.strip_prefix(CLDR_RULE_PREFIX) else {
                    continue;
                };
                if cat == OTHER {
                    continue;
                }
                let Some(raw) = val.as_str().and_then(cldr_to_rust) else {
                    continue;
                };
                out.push((cat.to_string(), raw));
            }
            if out.is_empty() {
                continue;
            }
            rules.entry(tag.clone()).or_insert(out);
            if let Some(base) = tag.split_once('-') {
                rules.entry(base.0.to_string()).or_default();
            }
        }
        rules
    })
}

fn cldr_to_rust(raw: &str) -> Option<String> {
    let rule = raw.split('@').next()?.trim();
    if rule.is_empty() {
        return None;
    }

    let mut or_terms = rule.split(OR_SEP).peekable();
    let mut out = Vec::new();

    while let Some(or_part) = or_terms.next() {
        match compile_or_part(or_part) {
            Some(expr) => out.push(expr),
            None => return None,
        }
    }

    if out.is_empty() {
        return None;
    }
    Some(out.join(" || "))
}

fn compile_or_part(s: &str) -> Option<String> {
    let mut and_terms = s.split(AND_SEP).peekable();
    let mut out = Vec::new();

    while let Some(rel) = and_terms.next() {
        let rel = rel.trim();
        if rel.is_empty() {
            continue;
        }
        if is_trivial(rel) {
            continue;
        }
        if uses_unsupported_var(rel) {
            return None;
        }

        let expanded = expand_relation(rel);
        if expanded.len() == 1 {
            out.push(expanded.into_iter().next().unwrap());
        } else {
            out.push(expanded.join(" || "));
        }
    }

    if out.is_empty() {
        return None;
    }
    let joined = out.join(" && ");
    Some(if and_terms.peek().is_some() {
        format!("({})", joined)
    } else {
        joined
    })
}

fn is_trivial(rel: &str) -> bool {
    let rel = rel.trim();
    rel == "v = 0" || rel == "w = 0" || rel == "f = 0" || rel == "t = 0"
}

fn uses_unsupported_var(rel: &str) -> bool {
    for var in &['v', 'w', 'f', 't', 'c', 'e'] {
        if is_standalone_var(rel, *var) {
            return true;
        }
    }
    false
}

fn is_standalone_var(s: &str, var: char) -> bool {
    let bytes = s.as_bytes();
    for i in 0..bytes.len() {
        if bytes[i] != var as u8 {
            continue;
        }
        let left_ok = i == 0 || bytes[i - 1] == b' ' || bytes[i - 1] == b'(';
        let right_ok = i + 1 >= bytes.len()
            || bytes[i + 1] == b' '
            || bytes[i + 1] == b'='
            || bytes[i + 1] == b'.'
            || bytes[i + 1] == b')';
        if left_ok && right_ok {
            return true;
        }
    }
    false
}

fn expand_relation(rel: &str) -> Vec<String> {
    let rel = normalize_var(rel);
    let (expr, op, values) = match parse_relation(&rel) {
        Some(t) => t,
        None => return vec![rel],
    };

    if !values.contains(',') {
        return vec![emit_value(expr, op, values)];
    }

    values
        .split(',')
        .map(|v| {
            let v = v.trim();
            if v.is_empty() {
                return String::new();
            }
            emit_value(expr, op, v)
        })
        .filter(|s| !s.is_empty())
        .collect()
}

fn normalize_var(s: &str) -> String {
    let s = s.replace(" i ", " n ");
    if let Some(rest) = s.strip_prefix("i ") {
        format!("n {}", rest)
    } else if s == "i" {
        "n".to_string()
    } else {
        s
    }
}

fn parse_relation(rel: &str) -> Option<(&str, &str, &str)> {
    const CLDR_NEQ: &str = " !=";
    const CLDR_EQ: &str = " =";
    if let Some(pos) = rel.find(CLDR_NEQ) {
        let (left, right) = rel.split_at(pos);
        Some((left.trim(), "!=", right[CLDR_NEQ.len()..].trim()))
    } else if let Some(pos) = rel.find(CLDR_EQ) {
        let (left, right) = rel.split_at(pos);
        Some((left.trim(), "==", right[CLDR_EQ.len()..].trim()))
    } else {
        None
    }
}

fn emit_value(expr: &str, op: &str, value: &str) -> String {
    if let Some((lo, hi)) = value.split_once(RANGE_SEP) {
        let lo = lo.trim();
        let hi = hi.trim();
        if op == "!=" {
            format!("!({} >= {} && {} <= {})", expr, lo, expr, hi)
        } else {
            format!("{} >= {} && {} <= {}", expr, lo, expr, hi)
        }
    } else {
        format!("{} {} {}", expr, op, value)
    }
}

pub fn plural_rule(lang: &str, cat: &str) -> Option<&'static str> {
    let rules = load_cldr();
    if let Some(entries) = rules.get(lang) {
        if let Some(pos) = entries.iter().position(|(c, _)| c == cat) {
            return Some(entries[pos].1.as_str());
        }
    }
    if let Some(base) = lang.split_once('-') {
        if let Some(entries) = rules.get(base.0) {
            if let Some(pos) = entries.iter().position(|(c, _)| c == cat) {
                return Some(entries[pos].1.as_str());
            }
        }
    }
    None
}

pub fn plural_rule_or_fallback(lang: &str, cat: &str) -> Option<&'static str> {
    plural_rule(lang, cat).or_else(|| match cat {
        "zero" => Some(FALLBACK_ZERO),
        "one" => Some(FALLBACK_ONE),
        "two" => Some(FALLBACK_TWO),
        _ => None,
    })
}

#[cfg(test)]
#[cfg_attr(coverage, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn plural_rules_basic() {
        assert_eq!(plural_rule("en", "one"), Some("n == 1"));
        assert_eq!(plural_rule("en", "few"), None);
        assert_eq!(plural_rule("fr", "one"), Some("n == 0 || n == 1"));
        assert_eq!(plural_rule("zh", "one"), None);
        assert_eq!(plural_rule("zh-CN", "one"), None);
    }

    #[test]
    fn plural_rules_fallback() {
        assert_eq!(plural_rule_or_fallback("xx", "one"), Some("n == 1"));
        assert_eq!(plural_rule_or_fallback("xx", "few"), None);
        assert_eq!(plural_rule_or_fallback("xx", "many"), None);
        assert_eq!(plural_rule_or_fallback("en", "few"), None);
    }

    #[test]
    fn plural_rules_complex() {
        assert_eq!(plural_rule("ar", "zero"), Some("n == 0"));
        assert_eq!(plural_rule("ar", "one"), Some("n == 1"));
        assert_eq!(plural_rule("ar", "two"), Some("n == 2"));
        let ar_few = plural_rule("ar", "few").unwrap();
        assert!(ar_few.contains("n % 100"));
        assert!(ar_few.contains("3"));
        assert!(ar_few.contains("10"));

        let pl_few = plural_rule("pl", "few").unwrap();
        assert!(pl_few.contains("n % 10"));
        assert!(pl_few.contains("2"));
        assert!(pl_few.contains("4"));

        let ru_many = plural_rule("ru", "many").unwrap();
        assert!(ru_many.contains("n % 10"));
        assert!(ru_many.contains("0") || ru_many.contains("5"));
    }

    #[test]
    fn plural_locale_fallback() {
        assert_eq!(plural_rule("zh-CN", "other"), None);
        assert_eq!(plural_rule("zh-CN", "one"), None);
    }

    #[test]
    fn plural_unsupported_language() {
        assert_eq!(plural_rule("xx", "one"), None);
        assert_eq!(plural_rule_or_fallback("xx", "one"), Some("n == 1"));
        assert_eq!(plural_rule_or_fallback("xx", "zero"), Some("n == 0"));
        assert_eq!(plural_rule_or_fallback("xx", "two"), Some("n == 2"));
        assert_eq!(plural_rule_or_fallback("xx", "few"), None);
        assert_eq!(plural_rule_or_fallback("xx", "many"), None);
    }

    #[test]
    fn normalize_var_cases() {
        assert_eq!(normalize_var("n"), "n");
        assert_eq!(normalize_var("n i"), "n i");
        assert_eq!(normalize_var("i 0"), "n 0");
        assert_eq!(normalize_var("i"), "n");
    }

    #[test]
    fn parse_relation_cases() {
        assert_eq!(parse_relation("n"), None);
        assert_eq!(parse_relation("n % 10"), None);
        let (expr, op, val) = parse_relation("n != 1").unwrap();
        assert_eq!(expr, "n");
        assert_eq!(op, "!=");
        assert_eq!(val, "1");
    }

    #[test]
    fn expand_relation_cases() {
        let result = expand_relation("n % 10");
        assert_eq!(result, vec!["n % 10"]);

        let result = expand_relation("n = 0..2");
        assert_eq!(result, vec!["n >= 0 && n <= 2"]);

        let result = expand_relation("n != 0..2");
        assert_eq!(result, vec!["!(n >= 0 && n <= 2)"]);

        let result = expand_relation("n = 1,2");
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], "n == 1");
        assert_eq!(result[1], "n == 2");

        // empty value in split
        let result = expand_relation("n = 1,,2");
        assert_eq!(result, vec!["n == 1", "n == 2"]);
    }

    #[test]
    fn cldr_to_rust_cases() {
        assert_eq!(cldr_to_rust("@metadata"), None);
        assert_eq!(cldr_to_rust("  "), None);
        assert_eq!(cldr_to_rust("n = 1 or v = 0"), None);
    }

    #[test]
    fn compile_or_part_cases() {
        assert_eq!(compile_or_part("v = 0"), None);
        assert_eq!(compile_or_part("n = 1"), Some("n == 1".to_string()));
        assert_eq!(compile_or_part(" and "), None);
        assert_eq!(compile_or_part("n = 1 and "), Some("n == 1".to_string()));
    }

    #[test]
    fn uses_unsupported_var_cases() {
        assert!(uses_unsupported_var("v = 5"));
        assert!(uses_unsupported_var("f != 0"));
        assert!(!uses_unsupported_var("n = 1"));
        assert!(!uses_unsupported_var("n % 10 = 0"));
    }

    #[test]
    fn is_standalone_var_cases() {
        assert!(is_standalone_var("v=0", 'v'));
        assert!(is_standalone_var("v.0", 'v'));
        assert!(is_standalone_var("(v)", 'v'));
        assert!(is_standalone_var("(v", 'v'));
        assert!(is_standalone_var("v", 'v'));
        assert!(is_standalone_var("v = 0", 'v'));
        assert!(!is_standalone_var("n = 1", 'v'));
        assert!(!is_standalone_var("abc", 'v'));
    }

    #[test]
    fn emit_value_cases() {
        assert_eq!(emit_value("n", "==", "1"), "n == 1");
        assert_eq!(emit_value("n", "!=", "0"), "n != 0");
        assert_eq!(emit_value("n", "==", "0..2"), "n >= 0 && n <= 2");
        assert_eq!(emit_value("n", "!=", "0..2"), "!(n >= 0 && n <= 2)");
    }
}
