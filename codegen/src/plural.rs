include!(concat!(env!("OUT_DIR"), "/plural_gen.rs"));

const FALLBACK_ZERO: &str = "n == 0";
const FALLBACK_ONE: &str = "n == 1";
const FALLBACK_TWO: &str = "n == 2";

pub fn plural_rule(lang: &str, cat: &str) -> Option<&'static str> {
    for locale in [Some(lang), lang.split_once('-').map(|(b, _)| b)]
        .into_iter()
        .flatten()
    {
        let entries = lookup_rules(locale)?;
        if let Some(pos) = entries.iter().position(|(c, _)| *c == cat) {
            return Some(entries[pos].1);
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
}
