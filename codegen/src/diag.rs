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
    for diag in diags {
        eprintln!("{}", diag.display());
        match diag.kind {
            DiagKind::Error => errors += 1,
            DiagKind::Warning => warnings += 1,
        }
    }
    if errors > 0 {
        panic!(
            "ftl-codegen: {} error(s), {} warning(s) — aborting",
            errors, warnings
        );
    }
}
