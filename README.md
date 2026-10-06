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

Download a binary from the [latest release](https://github.com/silexlabs/htmlrebase/releases/latest).
Linux only for now, on x64 and arm64. Nothing else to install.

```bash
sudo curl -fsSL https://github.com/silexlabs/htmlrebase/releases/latest/download/htmlrebase-linux-x64 \
  -o /usr/local/bin/htmlrebase && sudo chmod +x /usr/local/bin/htmlrebase
```

In a GitHub Actions workflow, after the build step:

```yaml
- run: |
    curl -fsSL https://github.com/silexlabs/htmlrebase/releases/latest/download/htmlrebase-linux-x64 -o htmlrebase && chmod +x htmlrebase
    ./htmlrebase _site --prefix /${{ github.event.repository.name }}/
```

Skip this step for an `owner.github.io` repository or a custom domain: the site is served at `/`.

## What it rewrites

In `.html` and `.css` files, every URL that starts with a single `/`: `href`, `src`,
`srcset`, `poster`, `action`, `<meta>` images, inline `style`, `<style>`, `<noscript>`,
`url()`, `image-set()` and `@import`. It leaves alone `//cdn…`, `https:`, `mailto:`,
`#anchors`, relative paths, and URLs that already start with the prefix, so running it twice
is safe. Apart from these URLs, the files keep their bytes, except that a rewritten attribute
comes out in double quotes.

Limits:

- A section of the site named like the prefix is not prefixed: with `--prefix /blog/`, a
  link to `/blog/post/` already looks done and stays as it is.
- JavaScript, XML (sitemap, RSS) and `.webmanifest` files are not rewritten.
- A file that is not UTF-8, or HTML too ambiguous to rewrite safely (a `<style>` inside a
  `<select>`), is skipped with a warning, and the command exits with an error once every
  other file is done.

## Performance

Measured on v0.1.0, on a Linux laptop (20 threads, SSD), with a site of 5,200 files and
122 MB (5,000 pages of 23 KB full of links and `srcset`, 200 CSS files):

- first run, every file rewritten: 10.2 s, 35% of one CPU, 5 MB of memory;
- second run, nothing to change: 0.7 s, one CPU.

The first run waits on the disk: each file is written to a temporary file, flushed with
`fsync`, then renamed. Ideas to make it faster, pull requests welcome:

- process files in parallel (it uses a single thread today);
- drop the `fsync` per file, or make it optional: the rename already protects against a
  crash of the process, `fsync` only adds safety against a power cut, rarely needed in CI;
- skip files that contain no `/` worth rewriting before running the HTML parser.

## Open to contributions

Builds for macOS and Windows, links inside JavaScript, SVG and XML files: pull requests are
welcome.

## License

GPL-3.0-only
