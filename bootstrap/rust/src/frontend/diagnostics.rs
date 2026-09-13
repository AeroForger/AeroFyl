use std::fmt;

use super::source::{SourceMap, Span};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Severity {
    Error,
    Warning,
    Note,
}

impl fmt::Display for Severity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Note => "note",
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostic {
    pub severity: Severity,
    pub message: String,
    pub span: Option<Span>,
}

impl Diagnostic {
    pub fn error(message: impl Into<String>, span: impl Into<Option<Span>>) -> Self {
        Self {
            severity: Severity::Error,
            message: message.into(),
            span: span.into(),
        }
    }

    pub fn render(&self, sources: &SourceMap) -> String {
        let Some(span) = self.span else {
            return format!("{}: {}", self.severity, self.message);
        };
        let Some(file) = sources.get(span.file) else {
            return format!(
                "{}: {} (invalid {})",
                self.severity, self.message, span.file
            );
        };
        let Some(location) = file.line_column(span.start) else {
            return format!(
                "{}: {} (invalid source offset {})",
                self.severity, self.message, span.start
            );
        };
        let line = file.line_text(location.line).unwrap_or("");
        let remaining_line = &file.text()[span.start..];
        let line_end = span.start + remaining_line.find('\n').unwrap_or(remaining_line.len());
        let marked_end = span.end.min(line_end);
        let marker_width = file.text()[span.start..marked_end].chars().count().max(1);
        format!(
            "{}:{}:{}: {}: {}\n  |\n{:>2} | {}\n  | {}{}",
            file.path().display(),
            location.line,
            location.column,
            self.severity,
            self.message,
            location.line,
            line,
            " ".repeat(location.column.saturating_sub(1)),
            "^".repeat(marker_width)
        )
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Diagnostics(Vec<Diagnostic>);

impl Diagnostics {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn push(&mut self, diagnostic: Diagnostic) {
        self.0.push(diagnostic);
    }
    pub fn extend(&mut self, diagnostics: impl IntoIterator<Item = Diagnostic>) {
        self.0.extend(diagnostics);
    }
    pub fn has_errors(&self) -> bool {
        self.0.iter().any(|item| item.severity == Severity::Error)
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    pub fn iter(&self) -> impl Iterator<Item = &Diagnostic> {
        self.0.iter()
    }
    pub fn into_vec(self) -> Vec<Diagnostic> {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::source::SourceMap;

    #[test]
    fn renders_source_location() {
        let mut sources = SourceMap::new();
        let file = sources.add("bad.fyl", "int value;\n");
        let rendered = Diagnostic::error("example", Span::new(file, 4, 9)).render(&sources);
        assert!(rendered.contains("bad.fyl:1:5: error: example"));
        assert!(rendered.contains("^^^^^"));
    }
}
