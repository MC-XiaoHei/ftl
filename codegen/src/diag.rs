use std::fmt::Write;

#[derive(Debug, Clone)]
pub struct Diag {
    pub file: String,
    pub locale: String,
    pub context: String,
    pub message: String,
    pub kind: DiagKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagKind {
    Error,
    Warning,
}

impl Diag {
    pub fn error(
        file: impl Into<String>,
        locale: impl Into<String>,
        context: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            file: file.into(),
            locale: locale.into(),
            context: context.into(),
            message: message.into(),
            kind: DiagKind::Error,
        }
    }

    pub fn warning(
        file: impl Into<String>,
        locale: impl Into<String>,
        context: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            file: file.into(),
            locale: locale.into(),
            context: context.into(),
            message: message.into(),
            kind: DiagKind::Warning,
        }
    }

    pub fn display(&self) -> String {
        let tag = match self.kind {
            DiagKind::Error => "error",
            DiagKind::Warning => "warning",
        };
        let mut s = String::new();
        if !self.file.is_empty() {
            write!(s, "{}:", self.file).unwrap();
        }
        if !self.locale.is_empty() {
            write!(s, " [{}]", self.locale).unwrap();
        }
        write!(s, " {}: {}", tag, self.message).unwrap();
        if !self.context.is_empty() {
            write!(s, " (context: {})", self.context).unwrap();
        }
        s
    }
}

pub fn report_diagnostics(diags: &[Diag]) {
    let mut errors = 0;
    let mut warnings = 0;
    let mut first_error = String::new();
    for diag in diags {
        let line = diag.display();
        eprintln!("{}", line);
        match diag.kind {
            DiagKind::Error => {
                if errors == 0 {
                    first_error = line;
                }
                errors += 1;
            }
            DiagKind::Warning => warnings += 1,
        }
    }
    if errors > 0 {
        panic!(
            "ftl-codegen: {} error(s), {} warning(s) — aborting\n{}",
            errors, warnings, first_error
        );
    }
}

#[cfg(test)]
#[cfg_attr(coverage, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn warning_constructor() {
        let d = Diag::warning("f.ftl", "en", "ctx", "msg");
        assert_eq!(d.kind, DiagKind::Warning);
        assert_eq!(d.file, "f.ftl");
        assert_eq!(d.locale, "en");
        assert_eq!(d.context, "ctx");
        assert_eq!(d.message, "msg");
    }

    #[test]
    fn display_error_full() {
        let d = Diag::error("f.ftl", "en", "ctx", "bad");
        let s = d.display();
        assert!(s.contains("f.ftl:"));
        assert!(s.contains("[en]"));
        assert!(s.contains("error"));
        assert!(s.contains("bad"));
        assert!(s.contains("ctx"));
    }

    #[test]
    fn display_warning_minimal() {
        let d = Diag::warning("", "", "", "warn");
        let s = d.display();
        assert!(!s.contains("["), "should not contain locale");
        assert!(s.starts_with(" warning"));
        assert!(s.contains("warn"));
    }

    #[test]
    fn report_warnings_only_does_not_panic() {
        let diags = vec![
            Diag::warning("", "", "", "w1"),
            Diag::warning("", "", "", "w2"),
        ];
        report_diagnostics(&diags);
    }

    #[test]
    #[should_panic(expected = "1 error(s), 2 warning")]
    fn report_mixed_panics_with_summary() {
        let diags = vec![
            Diag::warning("", "", "", "w1"),
            Diag::warning("", "", "", "w2"),
            Diag::error("f.ftl", "en", "", "e1"),
        ];
        report_diagnostics(&diags);
    }

    #[test]
    fn display_without_file() {
        let d = Diag::error("", "en", "", "msg");
        assert_eq!(d.display(), " [en] error: msg");
    }
}
