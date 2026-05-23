use std::fmt::Write;

pub fn escape_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => write!(out, "\\u{{{:04X}}}", c as u32).unwrap(),
            c => out.push(c),
        }
    }
    out
}

pub fn sanitize(s: &str) -> String {
    s.to_lowercase().replace('-', "_").replace('.', "_")
}

pub fn sanitize_upper(s: &str) -> String {
    s.split(|c: char| c == '-' || c == '.')
        .filter(|p| !p.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => {
                    first.to_uppercase().to_string()
                        + &chars.flat_map(|c| c.to_lowercase()).collect::<String>()
                }
            }
        })
        .collect()
}

pub fn sanitize_const(s: &str) -> String {
    s.split(|c: char| c == '-' || c == '.')
        .filter(|p| !p.is_empty())
        .map(|part| part.to_uppercase())
        .collect::<Vec<_>>()
        .join("_")
}

#[cfg(test)]
#[cfg_attr(coverage, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn sanitize_lower() {
        assert_eq!(sanitize("en"), "en");
        assert_eq!(sanitize("zh-CN"), "zh_cn");
        assert_eq!(sanitize("zh-TW"), "zh_tw");
        assert_eq!(sanitize("pt-BR"), "pt_br");
    }

    #[test]
    fn sanitize_pascal() {
        assert_eq!(sanitize_upper("en"), "En");
        assert_eq!(sanitize_upper("zh-CN"), "ZhCn");
        assert_eq!(sanitize_upper("zh-TW"), "ZhTw");
        assert_eq!(sanitize_upper("pt-BR"), "PtBr");
    }

    #[test]
    fn sanitize_screaming() {
        assert_eq!(sanitize_const("en"), "EN");
        assert_eq!(sanitize_const("zh-CN"), "ZH_CN");
        assert_eq!(sanitize_const("zh-TW"), "ZH_TW");
        assert_eq!(sanitize_const("pt-BR"), "PT_BR");
    }

    #[test]
    fn escape_text_cases() {
        assert_eq!(escape_str("hello"), "hello");
        assert_eq!(escape_str("he\"llo"), "he\\\"llo");
        assert_eq!(escape_str("a\nb"), "a\\nb");
        assert_eq!(escape_str("a\\b"), "a\\\\b");
        assert_eq!(escape_str("\t"), "\\t");
        assert_eq!(escape_str("\r"), "\\r");
        assert_eq!(escape_str("\x00"), "\\u{0000}");
        assert_eq!(escape_str("\x1b"), "\\u{001B}");
        assert_eq!(escape_str("\x7f"), "\\u{007F}");
    }

    #[test]
    fn sanitize_upper_edge_cases() {
        assert_eq!(sanitize_upper("zh-"), "Zh");
        assert_eq!(sanitize_upper("-cn"), "Cn");
        assert_eq!(sanitize_upper("--"), "");
    }
}
