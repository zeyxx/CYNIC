use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct NetworkDiagnostic {
    pub(crate) code: &'static str,
    pub(crate) category: &'static str,
    pub(crate) summary: &'static str,
}

pub(crate) fn diagnostic(
    code: &'static str,
    category: &'static str,
    summary: &'static str,
) -> NetworkDiagnostic {
    NetworkDiagnostic {
        code,
        category,
        summary,
    }
}
