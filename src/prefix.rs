use std::fmt;
use std::str::FromStr;

use crate::Error;

/// A base path, always stored as `/segment/…/`.
///
/// `repo`, `/repo` and `/repo/` all give `/repo/`. An empty prefix, or `/`, gives `/`, which
/// rewrites nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prefix(String);

impl Prefix {
    pub fn new(raw: &str) -> Result<Self, Error> {
        let trimmed = raw.trim().trim_matches('/');
        let invalid = |reason| Error::InvalidPrefix {
            prefix: raw.to_owned(),
            reason,
        };
        if trimmed.contains(':') {
            return Err(invalid("use a path such as /repo/, not a full URL"));
        }
        if trimmed.contains("//") {
            return Err(invalid("empty path segment"));
        }
        if trimmed
            .split('/')
            .any(|segment| segment == "." || segment == "..")
        {
            return Err(invalid("`.` and `..` are not allowed"));
        }
        if trimmed
            .chars()
            .any(|c| c.is_whitespace() || c.is_control() || "\"'<>()?#&\\".contains(c))
        {
            return Err(invalid(
                "spaces, quotes, brackets, parentheses, `?`, `#`, `&` and `\\` are not allowed",
            ));
        }
        if trimmed.is_empty() {
            Ok(Self("/".to_owned()))
        } else {
            Ok(Self(format!("/{trimmed}/")))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// `true` for `/`: there is nothing to rewrite.
    pub fn is_root(&self) -> bool {
        self.0 == "/"
    }

    pub(crate) fn without_trailing_slash(&self) -> &str {
        &self.0[..self.0.len() - 1]
    }
}

impl FromStr for Prefix {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl fmt::Display for Prefix {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes() {
        for raw in ["repo", "/repo", "/repo/", " repo/ "] {
            assert_eq!(Prefix::new(raw).unwrap().as_str(), "/repo/");
        }
        assert_eq!(Prefix::new("a/b").unwrap().as_str(), "/a/b/");
        assert!(Prefix::new("/").unwrap().is_root());
        assert!(Prefix::new("").unwrap().is_root());
    }

    #[test]
    fn rejects_urls_and_odd_characters() {
        assert!(Prefix::new("https://example.com/repo/").is_err());
        assert!(Prefix::new("/my repo/").is_err());
        assert!(Prefix::new("/repo?x").is_err());
        assert!(Prefix::new("/a/../b/").is_err());
        assert!(Prefix::new(".").is_err());
    }
}
