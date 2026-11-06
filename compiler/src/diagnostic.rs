use std::{ collections::HashMap, ops::Range };

use ariadne::{ Config, FnCache, Label, Report, ReportKind };
use typst::{
    diag::{ Severity, SourceDiagnostic },
    syntax::{ DiagSpan, DiagSpanKind, FileId },
};

use crate::file_entry::FileEntry;

#[derive(Hash, PartialEq, Eq, Clone, Copy)]
struct Id(Option<FileId>);

impl std::fmt::Debug for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0.is_some() { write!(f, "{:?}", self.0.unwrap()) } else { write!(f, "") }
    }
}

impl std::fmt::Display for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0.is_some() { write!(f, "{:?}", self.0.unwrap()) } else { write!(f, "") }
    }
}

pub fn format_diagnostic(
    sources: &HashMap<FileId, FileEntry>,
    diagnostics: &[SourceDiagnostic]
) -> String {
    let mut bytes = Vec::new();

    let mut cache = FnCache::new(
        |id: &Id| -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
            Ok(
                id.0
                    .and_then(|id| sources.get(&id))
                    .map(|entry| entry.source().text().to_string())
                    .unwrap_or_default()
            )
        }
    );

    for diagnostic in diagnostics {
        let mut report = build_report(
            diagnostic.span,
            match diagnostic.severity {
                Severity::Error => ReportKind::Error,
                Severity::Warning => ReportKind::Warning,
            },
            diagnostic.message.to_string(),
            sources
        );

        if !diagnostic.hints.is_empty() {
            let hints: Vec<String> = diagnostic
                .hints
                .iter()
                .map(|hint| format!("hint: {}", hint.v))
                .collect();
            report.set_help(hints.join("\n"));
        }
        report.finish().write(&mut cache, &mut bytes).unwrap();

        bytes.push(b'\n');
        for point in &diagnostic.trace {
            build_report(point.span.into(), ReportKind::Advice, point.v.to_string(), sources)
                .finish()
                .write(&mut cache, &mut bytes)
                .unwrap();

            bytes.push(b'\n');
        }
    }

    return String::from_utf8(bytes).unwrap().trim().to_string();
}

fn build_report<'a>(
    span: DiagSpan,
    report_kind: ReportKind<'a>,
    message: String,
    sources: &HashMap<FileId, FileEntry>
) -> ariadne::ReportBuilder<'a, (Id, Range<usize>)> {
    let config = Config::default().with_color(false).with_tab_width(2);
    let (id, range) = match span.get() {
        DiagSpanKind::Detached => (Id(None), 0..0),
        DiagSpanKind::Range { id, range } => (Id(Some(id)), range),
        DiagSpanKind::Number { id, num, sub_range } => {
            let range = sources
                .get(&id)
                .and_then(|entry| entry.source().range(num, sub_range))
                .unwrap_or(0..0);
            (Id(Some(id)), range)
        }
    };

    Report::build(report_kind, (id, range.clone()))
        .with_config(config)
        .with_message(message)
        .with_label(Label::new((id, range)))
}
