//! Add a base URL to a static site that is already built.
//!
//! A static site generator writes root-relative URLs such as `/css/a.css`. Served from a
//! sub-folder (`https://owner.codeberg.page/repo/`), they break. This crate rewrites them in
//! the HTML and CSS files, so that `/css/a.css` becomes `/repo/css/a.css`.
//!
//! ```
//! use htmlrebase::{rebase_css, rebase_html, Prefix};
//!
//! let prefix = Prefix::new("repo").unwrap();
//! assert_eq!(prefix.as_str(), "/repo/");
//!
//! let html = rebase_html(r#"<img src="/a.png">"#, &prefix).unwrap();
//! assert_eq!(html, r#"<img src="/repo/a.png">"#);
//!
//! assert_eq!(rebase_css("a { background: url(/a.png) }", &prefix), "a { background: url(/repo/a.png) }");
//! ```
//!
//! Only URLs that start with a single `/` are rewritten. A URL that already starts with the
//! prefix is left alone, so running twice gives the same result as running once.

mod css;
mod errors;
mod html;
mod prefix;

use std::fs;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

pub use css::rebase_css;
pub use errors::Error;
pub use html::rebase_html;
pub use prefix::Prefix;

/// What [`rebase_dir`] did.
#[derive(Debug, Default)]
pub struct Stats {
    /// HTML and CSS files read.
    pub scanned: usize,
    /// Files rewritten on disk, in the order they were found.
    pub changed: Vec<PathBuf>,
}

/// Rewrite one URL, or return `None` when it must stay as it is.
///
/// ```
/// use htmlrebase::{rebase_url, Prefix};
///
/// let prefix = Prefix::new("/repo/").unwrap();
/// assert_eq!(rebase_url("/about/", &prefix).as_deref(), Some("/repo/about/"));
/// assert_eq!(rebase_url("/repo/about/", &prefix), None);
/// assert_eq!(rebase_url("//cdn.example.com/x.js", &prefix), None);
/// assert_eq!(rebase_url("about/", &prefix), None);
/// ```
pub fn rebase_url(url: &str, prefix: &Prefix) -> Option<String> {
    if prefix.is_root() {
        return None;
    }
    let rest = url.strip_prefix('/')?;
    // Browsers read `/\host` as `//host`.
    if rest.starts_with(['/', '\\']) {
        return None;
    }
    if let Some(after) = url.strip_prefix(prefix.without_trailing_slash()) {
        if after.is_empty() || after.starts_with(['/', '?', '#']) {
            return None;
        }
    }
    Some(format!("{}{rest}", prefix.as_str()))
}

/// Rewrite every `.html`, `.htm` and `.css` file under `dir`, in place.
///
/// Files are only written when their content changes. Symbolic links are not followed.
pub fn rebase_dir(dir: &Path, prefix: &Prefix) -> Result<Stats, Error> {
    if !dir.is_dir() {
        return Err(Error::NotADirectory(dir.to_path_buf()));
    }
    let mut stats = Stats::default();
    for entry in WalkDir::new(dir).sort_by_file_name() {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        let Some(kind) = FileKind::of(path) else {
            continue;
        };
        stats.scanned += 1;
        let io_error = |source| Error::Io {
            path: path.to_path_buf(),
            source,
        };
        let source = fs::read_to_string(path).map_err(io_error)?;
        let rebased = match kind {
            FileKind::Html => rebase_html(&source, prefix).map_err(|source| Error::HtmlFile {
                path: path.to_path_buf(),
                source: Box::new(source),
            })?,
            FileKind::Css => rebase_css(&source, prefix),
        };
        if rebased != source {
            fs::write(path, rebased).map_err(io_error)?;
            stats.changed.push(path.to_path_buf());
        }
    }
    Ok(stats)
}

enum FileKind {
    Html,
    Css,
}

impl FileKind {
    fn of(path: &Path) -> Option<Self> {
        let ext = path.extension()?.to_str()?.to_ascii_lowercase();
        match ext.as_str() {
            "html" | "htm" => Some(Self::Html),
            "css" => Some(Self::Css),
            _ => None,
        }
    }
}
