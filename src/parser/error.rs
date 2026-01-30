use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParserError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("syntax error: {0}")]
    Syntax(String),

    #[error("{0}")]
    Other(String),
}
