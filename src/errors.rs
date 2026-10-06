use std::path::PathBuf;

use lol_html::errors::RewritingError;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid prefix {prefix:?}: {reason}")]
    InvalidPrefix {
        prefix: String,
        reason: &'static str,
    },

    #[error("{0}: not a directory")]
    NotADirectory(PathBuf),

    #[error("not valid UTF-8")]
    NotUtf8,

    #[error("has other hard links, which would change too")]
    HardLinked,

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Html(#[from] RewritingError),
}
