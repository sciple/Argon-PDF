use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(tag = "type", content = "message")]
pub enum AppError {
    Pdf(String),
    DocNotFound(String),
    Db(String),
    Io(String),
    NeedsPassword,
    InvalidPdf,
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pdf(s) => write!(f, "PDF error: {s}"),
            Self::DocNotFound(s) => write!(f, "Document not found: {s}"),
            Self::Db(s) => write!(f, "Database error: {s}"),
            Self::Io(s) => write!(f, "IO error: {s}"),
            Self::NeedsPassword => write!(f, "Document requires a password"),
            Self::InvalidPdf => write!(f, "Not a valid PDF file"),
        }
    }
}

impl std::error::Error for AppError {}

impl From<rusqlite::Error> for AppError {
    fn from(e: rusqlite::Error) -> Self {
        Self::Db(e.to_string())
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e.to_string())
    }
}
