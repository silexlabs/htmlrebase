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

    #[error(transparent)]
    Walk(#[from] walkdir::Error),

    #[error("{path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("cannot rewrite the HTML: {0}")]
    Html(#[source] RewritingError),

    #[error("{path}: {source}")]
    HtmlFile {
        path: PathBuf,
        #[source]
        source: Box<Error>,
    },
}
