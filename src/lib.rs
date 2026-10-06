//! Add a base URL to a static site that is already built.
//!
//! A static site generator writes root-relative URLs such as `/css/a.css`. Served from a
//! sub-folder (`https://owner.codeberg.page/repo/`), they break. This crate, the core of
//! the `htmlrebase` binary, rewrites them in the HTML and CSS files, so that `/css/a.css` becomes `/repo/css/a.css`.
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

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

pub use css::rebase_css;
pub use errors::Error;
pub use html::rebase_html;
pub use prefix::Prefix;

/// What [`rebase_dir`] did.
#[derive(Debug, Default)]
pub struct Stats {
    /// HTML and CSS files found.
    pub scanned: usize,
    /// Files rewritten on disk, in the order they were found.
    pub changed: Vec<PathBuf>,
    /// Files and folders left as they were, and why.
    pub skipped: Vec<(PathBuf, Error)>,
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
/// Files are only written when their content changes, through a temporary file renamed over
/// the original. Symbolic links are not followed. A file that cannot be read or written is
/// skipped and listed in [`Stats::skipped`], and the walk goes on.
pub fn rebase_dir(dir: &Path, prefix: &Prefix) -> Result<Stats, Error> {
    if !dir.is_dir() {
        return Err(Error::NotADirectory(dir.to_path_buf()));
    }
    let mut stats = Stats::default();
    if prefix.is_root() {
        return Ok(stats);
    }
    for entry in WalkDir::new(dir).sort_by_file_name() {
        let entry = match entry {
            Ok(entry) => entry,
            Err(err) => {
                let path = err.path().unwrap_or(dir).to_path_buf();
                // Without following links, walkdir only fails on I/O.
                let err = err
                    .into_io_error()
                    .unwrap_or_else(|| io::Error::other("file system loop"));
                stats.skipped.push((path, Error::Io(err)));
                continue;
            }
        };
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        let Some(kind) = FileKind::of(path) else {
            continue;
        };
        stats.scanned += 1;
        match rebase_file(path, kind, prefix) {
            Ok(true) => stats.changed.push(path.to_path_buf()),
            Ok(false) => {}
            Err(err) => stats.skipped.push((path.to_path_buf(), err)),
        }
    }
    Ok(stats)
}

/// Returns whether the file changed.
fn rebase_file(path: &Path, kind: FileKind, prefix: &Prefix) -> Result<bool, Error> {
    let source = String::from_utf8(fs::read(path)?).map_err(|_| Error::NotUtf8)?;
    let rebased = match kind {
        FileKind::Html => rebase_html(&source, prefix)?,
        FileKind::Css => rebase_css(&source, prefix),
    };
    if rebased == source {
        return Ok(false);
    }
    write_atomically(path, rebased.as_bytes())?;
    Ok(true)
}

/// A crash or a full disk leaves either the old file or the new one, never half of it.
fn write_atomically(path: &Path, content: &[u8]) -> Result<(), Error> {
    let metadata = fs::symlink_metadata(path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if metadata.nlink() > 1 {
            return Err(Error::HardLinked);
        }
    }
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    let temp = path.with_file_name(format!(".{name}.htmlrebase-{}", std::process::id()));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)?;
    let written = file
        .write_all(content)
        .and_then(|()| file.set_permissions(metadata.permissions()))
        .and_then(|()| file.sync_all())
        .and_then(|()| fs::rename(&temp, path));
    if written.is_err() {
        let _ = fs::remove_file(&temp);
    }
    Ok(written?)
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
