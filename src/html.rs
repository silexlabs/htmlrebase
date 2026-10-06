use std::cell::RefCell;
use std::sync::LazyLock;

use lol_html::html_content::{ContentType, Element};
use lol_html::{element, rewrite_str, text, RewriteStrSettings};
use regex::Regex;

use crate::{rebase_css, rebase_url, Error, Prefix};

const URL_ATTRIBUTES: &[&str] = &[
    "href",
    "src",
    "poster",
    "action",
    "formaction",
    "xlink:href",
];
const SRCSET_ATTRIBUTES: &[&str] = &["srcset", "imagesrcset"];
const META_URL_PROPERTIES: &[&str] = &[
    "og:image",
    "og:image:url",
    "og:image:secure_url",
    "og:video",
    "og:audio",
    "og:url",
    "twitter:image",
    "twitter:image:src",
];

static REFRESH: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^\s*[\d.]+\s*[;,]\s*(?i:url\s*=\s*)?['"]?"#).expect("valid regex")
});

/// Rewrite root-relative URLs in an HTML document.
///
/// Covers `href`, `src`, `srcset`, `poster`, `action`, `formaction`, `<object data>`,
/// `<meta content>` for Open Graph and Twitter images and for `refresh`, `style="…"` and
/// `<style>` blocks. Everything else comes out byte for byte as it went in.
///
/// A rewritten attribute comes out double-quoted, whatever its original quoting.
pub fn rebase_html(html: &str, prefix: &Prefix) -> Result<String, Error> {
    if prefix.is_root() {
        return Ok(html.to_owned());
    }
    let style = RefCell::new(String::new());
    let settings = RewriteStrSettings::new()
        .with_enable_esi_tags(false)
        .append_element_content_handler(element!("*", |el| {
            rebase_element(el, prefix);
            Ok(())
        }))
        .append_element_content_handler(text!("style", |chunk| {
            let mut buffer = style.borrow_mut();
            buffer.push_str(chunk.as_str());
            if chunk.last_in_text_node() {
                chunk.replace(&rebase_css(&buffer, prefix), ContentType::Html);
                buffer.clear();
            } else {
                chunk.remove();
            }
            Ok(())
        }));
    rewrite_str(html, settings).map_err(Error::Html)
}

fn rebase_element(el: &mut Element, prefix: &Prefix) {
    let tag = el.tag_name();
    let attributes: Vec<(String, String)> = el
        .attributes()
        .iter()
        .map(|a| (a.name(), a.value()))
        .collect();
    for (name, value) in &attributes {
        let rebased = match name.as_str() {
            n if URL_ATTRIBUTES.contains(&n) => rebase_trimmed(value, prefix),
            n if SRCSET_ATTRIBUTES.contains(&n) => rebase_srcset(value, prefix),
            "data" if tag == "object" => rebase_trimmed(value, prefix),
            "style" => Some(rebase_css(value, prefix)).filter(|css| css != value),
            "content" if tag == "meta" => rebase_meta(el, value, prefix),
            _ => None,
        };
        if let Some(rebased) = rebased {
            // The name was read from this very tag, so it is a valid attribute name.
            let _ = el.set_attribute(name, &rebased);
        }
    }
}

fn rebase_meta(el: &Element, content: &str, prefix: &Prefix) -> Option<String> {
    let is_url = ["property", "name"].iter().any(|attr| {
        el.get_attribute(attr)
            .is_some_and(|p| META_URL_PROPERTIES.contains(&p.trim().to_ascii_lowercase().as_str()))
    });
    if is_url {
        return rebase_trimmed(content, prefix);
    }
    let is_refresh = el
        .get_attribute("http-equiv")
        .is_some_and(|v| v.trim().eq_ignore_ascii_case("refresh"));
    if !is_refresh {
        return None;
    }
    let start = REFRESH.find(content)?.end();
    let tail = &content[start..];
    let end = tail.find(['"', '\'']).unwrap_or(tail.len());
    let url = tail[..end].trim_end();
    let rebased = rebase_url(url, prefix)?;
    Some(format!(
        "{}{rebased}{}",
        &content[..start],
        &content[start + url.len()..]
    ))
}

/// Browsers ignore whitespace around a URL attribute, so `href=" /a"` is a root-relative URL.
fn rebase_trimmed(value: &str, prefix: &Prefix) -> Option<String> {
    let trimmed = value.trim_start();
    let lead = &value[..value.len() - trimmed.len()];
    rebase_url(trimmed, prefix).map(|url| format!("{lead}{url}"))
}

/// Follows the candidate splitting of the HTML spec: a URL ends at whitespace, and commas
/// stuck to its end are separators, not part of it.
fn rebase_srcset(value: &str, prefix: &Prefix) -> Option<String> {
    let bytes = value.as_bytes();
    let mut out = String::with_capacity(value.len() + 16);
    let mut changed = false;
    let mut i = 0;
    while i < bytes.len() {
        let start = i;
        while i < bytes.len() && (bytes[i].is_ascii_whitespace() || bytes[i] == b',') {
            i += 1;
        }
        out.push_str(&value[start..i]);

        let url_start = i;
        while i < bytes.len() && !bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        let mut url_end = i;
        while url_end > url_start && bytes[url_end - 1] == b',' {
            url_end -= 1;
        }
        let url = &value[url_start..url_end];
        match rebase_url(url, prefix) {
            Some(rebased) => {
                out.push_str(&rebased);
                changed = true;
            }
            None => out.push_str(url),
        }
        out.push_str(&value[url_end..i]);

        if url_end == i {
            let descriptors = i;
            while i < bytes.len() && bytes[i] != b',' {
                i += 1;
            }
            out.push_str(&value[descriptors..i]);
        }
    }
    changed.then_some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rebase(html: &str) -> String {
        rebase_html(html, &Prefix::new("/repo/").unwrap()).unwrap()
    }

    #[test]
    fn srcset_rewrites_every_candidate() {
        assert_eq!(
            rebase(r#"<img srcset="/a.png 1x,/b.png 2x, https://x/c.png 3x,  /d.png">"#),
            r#"<img srcset="/repo/a.png 1x,/repo/b.png 2x, https://x/c.png 3x,  /repo/d.png">"#
        );
        assert_eq!(
            rebase(r#"<source srcset="/a.png,  /b.png 2x">"#),
            r#"<source srcset="/repo/a.png,  /repo/b.png 2x">"#
        );
    }

    #[test]
    fn inline_style_and_style_blocks() {
        assert_eq!(
            rebase(r#"<div style="background:url('/bg.png')"></div>"#),
            r#"<div style="background:url('/repo/bg.png')"></div>"#
        );
        assert_eq!(
            rebase("<style>@import '/a.css';\nb{background:url(/b.png)}</style>"),
            "<style>@import '/repo/a.css';\nb{background:url(/repo/b.png)}</style>"
        );
    }

    #[test]
    fn meta_tags() {
        assert_eq!(
            rebase(
                r#"<meta property="og:image" content="/og.png"><meta name="description" content="/not-a-url">"#
            ),
            r#"<meta property="og:image" content="/repo/og.png"><meta name="description" content="/not-a-url">"#
        );
        assert_eq!(
            rebase(r#"<meta http-equiv="refresh" content="0; url=/new/">"#),
            r#"<meta http-equiv="refresh" content="0; url=/repo/new/">"#
        );
    }

    #[test]
    fn object_data_only() {
        assert_eq!(
            rebase(r#"<object data="/a.svg"></object><div data="/x"></div>"#),
            r#"<object data="/repo/a.svg"></object><div data="/x"></div>"#
        );
    }

    #[test]
    fn untouched_bytes_stay_identical() {
        let html = "<!DOCTYPE html>\n<HTML lang=fr><!-- href=\"/x\" -->\n<P class='a'  id=b>caf&eacute; &amp; /x</P>\n<a href=\"//cdn.x/a\" title='t'>x</a><a href=#top>t</a><a href=\"mailto:a@b\">m</a>\n<script>var u = \"/x\";</script><br/>\n";
        assert_eq!(rebase(html), html);
    }

    #[test]
    fn entities_inside_a_rewritten_url_are_kept() {
        assert_eq!(
            rebase(r#"<a href="/s?a=1&amp;b=2">s</a>"#),
            r#"<a href="/repo/s?a=1&amp;b=2">s</a>"#
        );
    }
}
