# htmlrebase

Add a base URL to a site built by any static site generator.

Your generator writes links like `/css/a.css`. Served from a sub-folder, such as
`https://owner.codeberg.page/repo/`, they break. `htmlrebase` fixes the built files in place:

```bash
htmlrebase ./_site --prefix /repo/
```

```
/css/a.css   →   /repo/css/a.css
```

It works with any static site generator (Eleventy, Hugo, Jekyll, Zola, Silex) because it
only reads and writes files. Run it after the build.

## Install

Download a binary from the [releases page](https://github.com/silexlabs/htmlrebase/releases).
Linux only for now, on x64 and arm64. Nothing else to install.

```bash
curl -fsSL https://github.com/silexlabs/htmlrebase/releases/latest/download/htmlrebase-linux-x64 \
  -o /usr/local/bin/htmlrebase && chmod +x /usr/local/bin/htmlrebase
```

## What it rewrites

In `.html` and `.css` files, every URL that starts with a single `/`: `href`, `src`,
`srcset`, `poster`, `action`, `<meta>` images, inline `style`, `<style>`, `url()` and
`@import`. It leaves alone `//cdn…`, `https:`, `mailto:`, `#anchors`, relative paths, and
URLs that already start with the prefix, so running it twice is safe. Nothing else in the
files changes.

It is also a Rust library: `rebase_html`, `rebase_css`, `rebase_dir`.

## Open to contributions

Builds for macOS and Windows, links inside JavaScript, SVG and XML files: pull requests are
welcome.

## License

GPL-3.0
