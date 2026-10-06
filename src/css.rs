use std::sync::LazyLock;

use regex::Regex;

use crate::{rebase_url, Prefix};

static CSS_URL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?x)
        (?i:url) \( \s* (?: "([^"]*)" | '([^']*)' | ([^"'\s)][^\s)]*) )
        | @(?i:import) \s+ (?: "([^"]*)" | '([^']*)' )
        "#,
    )
    .expect("valid regex")
});

/// Rewrite `url(/…)` and `@import "/…"` in a stylesheet.
///
/// The rest of the text is returned unchanged.
pub fn rebase_css(css: &str, prefix: &Prefix) -> String {
    if prefix.is_root() {
        return css.to_owned();
    }
    let mut out = String::with_capacity(css.len());
    let mut last = 0;
    for caps in CSS_URL.captures_iter(css) {
        let Some(url) = caps.iter().skip(1).flatten().next() else {
            continue;
        };
        if let Some(rebased) = rebase_url(url.as_str(), prefix) {
            out.push_str(&css[last..url.start()]);
            out.push_str(&rebased);
            last = url.end();
        }
    }
    out.push_str(&css[last..]);
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
    fn leaves_other_urls_alone() {
        let css = r##"a{b:url(//cdn.x/a.png),url(https://x/a.png),url(data:image/png;base64,AA==),url(x.png),url("#f"),url(/repo/x.png)}"##;
        assert_eq!(rebase(css), css);
    }
}
