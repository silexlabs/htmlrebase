use std::sync::LazyLock;

use regex::Regex;

use crate::{rebase_url, Prefix};

// `image-set()` takes bare strings as URLs. One level of nested parentheses covers
// `url()` and `type()` inside it; deeper nesting falls back to the `url()` branch.
static CSS_URL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?x)
        (?i:image-set) \s* \( (?P<set> [^()]* (?: \( [^()]* \) [^()]* )* ) \)
        | (?: ^ | [^\w-] ) (?i:url) \( \s* (?: "([^"]*)" | '([^']*)' | ([^"'\s)][^\s)]*) )
        | @(?i:import) \s* (?: "([^"]*)" | '([^']*)' )
        "#,
    )
    .expect("valid regex")
});

static IMAGE_SET_URL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?x)
        "([^"]*)" | '([^']*)'
        | (?: ^ | [^\w-] ) (?i:url) \( \s* ([^"'\s)][^\s)]*)
        "#,
    )
    .expect("valid regex")
});

/// Rewrite `url(/…)`, `@import "/…"` and `image-set("/…")` in a stylesheet.
///
/// The rest of the text is returned unchanged.
pub fn rebase_css(css: &str, prefix: &Prefix) -> String {
    let points: Vec<usize> = url_starts(css, prefix).iter().map(|s| s + 1).collect();
    insert_prefix(css, &points, prefix_tail(prefix))
}

/// Byte offsets of the leading `/` of every URL in `css` that needs the prefix.
pub(crate) fn url_starts(css: &str, prefix: &Prefix) -> Vec<usize> {
    let mut starts = Vec::new();
    if prefix.is_root() {
        return starts;
    }
    let mut keep = |url: regex::Match, offset: usize| {
        if rebase_url(url.as_str(), prefix).is_some() {
            starts.push(offset + url.start());
        }
    };
    for caps in CSS_URL.captures_iter(css) {
        if let Some(set) = caps.name("set") {
            for item in IMAGE_SET_URL.captures_iter(set.as_str()) {
                if let Some(url) = item.iter().skip(1).flatten().next() {
                    keep(url, set.start());
                }
            }
        } else if let Some(url) = caps.iter().skip(1).flatten().next() {
            keep(url, 0);
        }
    }
    starts
}

/// What goes after the leading `/` of a URL: `repo/` for the prefix `/repo/`.
pub(crate) fn prefix_tail(prefix: &Prefix) -> &str {
    &prefix.as_str()[1..]
}

pub(crate) fn insert_prefix(text: &str, points: &[usize], tail: &str) -> String {
    let mut out = String::with_capacity(text.len() + points.len() * tail.len());
    let mut last = 0;
    for &at in points {
        out.push_str(&text[last..at]);
        out.push_str(tail);
        last = at;
    }
    out.push_str(&text[last..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rebase(css: &str) -> String {
        rebase_css(css, &Prefix::new("/repo/").unwrap())
    }

    #[test]
    fn every_quoting_and_spacing() {
        assert_eq!(rebase("a{b:url(/x.png)}"), "a{b:url(/repo/x.png)}");
        assert_eq!(rebase("a{b:url( /x.png )}"), "a{b:url( /repo/x.png )}");
        assert_eq!(
            rebase(r#"a{b:url("/x.png")}"#),
            r#"a{b:url("/repo/x.png")}"#
        );
        assert_eq!(
            rebase("a{b:url( '/x y.png' )}"),
            "a{b:url( '/repo/x y.png' )}"
        );
        assert_eq!(rebase("a{b:URL(/x.png)}"), "a{b:URL(/repo/x.png)}");
        assert_eq!(
            rebase(r#"@import "/a.css"; @import '/b.css'; @import url(/c.css);"#),
            r#"@import "/repo/a.css"; @import '/repo/b.css'; @import url(/repo/c.css);"#
        );
    }

    #[test]
    fn minified_import_and_image_set() {
        assert_eq!(
            rebase(r#"@import"/a.css";@import'/b.css';"#),
            r#"@import"/repo/a.css";@import'/repo/b.css';"#
        );
        assert_eq!(
            rebase(r#"a{b:image-set("/a.png" 1x,url(/b.png) 2x,"/c.avif" type("image/avif"))}"#),
            r#"a{b:image-set("/repo/a.png" 1x,url(/repo/b.png) 2x,"/repo/c.avif" type("image/avif"))}"#
        );
    }

    #[test]
    fn leaves_other_urls_alone() {
        let css = r##"a{b:url(//cdn.x/a.png),url(https://x/a.png),url(data:image/png;base64,AA==),url(x.png),url("#f"),url(/repo/x.png),--myurl(/x)}"##;
        assert_eq!(rebase(css), css);
    }
}
