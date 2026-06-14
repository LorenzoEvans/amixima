use std::fmt;
use std::path::PathBuf;

pub type Result<T> = std::result::Result<T, AmiximaError>;

#[derive(Debug)]
#[allow(dead_code)]
pub enum AmiximaError {
    Io {
        path: Option<PathBuf>,
        source: std::io::Error,
    },
    Parser(String),
    Validation(String),
    UnsupportedFormat(String),
    AudioDecode(String),
    AudioEncode(String),
    Processing(String),
    Batch(String),
    Path(String),
}

impl AmiximaError {
    pub fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::Io {
            path: Some(path.into()),
            source,
        }
    }
}

impl fmt::Display for AmiximaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, source } => {
                if let Some(path) = path {
                    write!(f, "I/O error at {}: {source}", path.display())
                } else {
                    write!(f, "I/O error: {source}")
                }
            }
            Self::Parser(message) => write!(f, "parser error: {message}"),
            Self::Validation(message) => write!(f, "validation error: {message}"),
            Self::UnsupportedFormat(message) => write!(f, "unsupported format: {message}"),
            Self::AudioDecode(message) => write!(f, "audio decode error: {message}"),
            Self::AudioEncode(message) => write!(f, "audio encode error: {message}"),
            Self::Processing(message) => write!(f, "processing error: {message}"),
            Self::Batch(message) => write!(f, "batch error: {message}"),
            Self::Path(message) => write!(f, "path error: {message}"),
        }
    }
}

impl std::error::Error for AmiximaError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

impl From<std::io::Error> for AmiximaError {
    fn from(source: std::io::Error) -> Self {
        Self::Io { path: None, source }
    }
}

impl From<serde_json::Error> for AmiximaError {
    fn from(source: serde_json::Error) -> Self {
        Self::Parser(source.to_string())
    }
}
