use std::fs;
use std::path::{Path, PathBuf};

use htmlrebase::{rebase_dir, Error, Prefix};

fn site(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("htmlrebase-test-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("css")).unwrap();
    fs::create_dir_all(dir.join("about")).unwrap();
    fs::write(
        dir.join("index.html"),
        r#"<link rel="stylesheet" href="/css/a.css"><a href="/about/">About</a><img src="/a.png" srcset="/a.png 1x, /a@2x.png 2x">"#,
    )
    .unwrap();
    fs::write(dir.join("about/index.htm"), r#"<a href="/">Home</a>"#).unwrap();
    fs::write(dir.join("css/a.css"), "body { background: url('/a.png') }").unwrap();
    fs::write(dir.join("notes.txt"), "/not/touched").unwrap();
    dir
}

fn read(dir: &Path, file: &str) -> String {
    fs::read_to_string(dir.join(file)).unwrap()
}

#[test]
fn rewrites_a_site_and_a_second_run_changes_nothing() {
    let dir = site("idempotent");
    let prefix = Prefix::new("repo").unwrap();

    let stats = rebase_dir(&dir, &prefix).unwrap();
    assert_eq!(stats.scanned, 3);
    assert_eq!(stats.changed.len(), 3);
    assert_eq!(
        read(&dir, "index.html"),
        r#"<link rel="stylesheet" href="/repo/css/a.css"><a href="/repo/about/">About</a><img src="/repo/a.png" srcset="/repo/a.png 1x, /repo/a@2x.png 2x">"#
    );
    assert_eq!(
        read(&dir, "about/index.htm"),
        r#"<a href="/repo/">Home</a>"#
    );
    assert_eq!(
        read(&dir, "css/a.css"),
        "body { background: url('/repo/a.png') }"
    );
    assert_eq!(read(&dir, "notes.txt"), "/not/touched");

    let again = rebase_dir(&dir, &prefix).unwrap();
    assert!(again.changed.is_empty());

    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn missing_directory_is_a_clear_error() {
    let err = rebase_dir(
        Path::new("/no/such/htmlrebase/dir"),
        &Prefix::new("/r/").unwrap(),
    )
    .unwrap_err();
    assert!(matches!(err, Error::NotADirectory(_)));
    assert_eq!(err.to_string(), "/no/such/htmlrebase/dir: not a directory");
}

#[test]
fn a_file_that_is_not_utf8_is_skipped_and_the_rest_is_rewritten() {
    let dir = site("latin1");
    fs::write(dir.join("about/latin1.html"), b"<a href=\"/x\">caf\xe9</a>").unwrap();

    let stats = rebase_dir(&dir, &Prefix::new("repo").unwrap()).unwrap();
    assert_eq!(stats.scanned, 4);
    assert_eq!(stats.changed.len(), 3);
    assert_eq!(stats.skipped.len(), 1);
    assert!(matches!(stats.skipped[0], (ref p, Error::NotUtf8) if p.ends_with("latin1.html")));
    assert_eq!(
        read(&dir, "css/a.css"),
        "body { background: url('/repo/a.png') }"
    );
    assert_eq!(
        fs::read(dir.join("about/latin1.html")).unwrap(),
        b"<a href=\"/x\">caf\xe9</a>"
    );

    fs::remove_dir_all(&dir).unwrap();
}
