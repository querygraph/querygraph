# QueryGraph Stack Publishing Workflow

This directory is the reference publishing workflow for QueryGraph-family repos.
It consolidates the working practices that began in `~/src/typesec` and were
extended through `~/src/lakecat`, `~/src/querygraph`, `~/src/omnighost`, Grust,
and the local books archive.

Use it as the source template when a repo needs a book/blog pipeline with:

- stable PDF/EPUB/MOBI artifacts;
- versioned delivery names such as `typesec (0.12.0-a1b2c3).epub`;
- `VERSION.md` as the delivery truth source;
- Mermaid diagrams rendered to persistent `.png` files;
- Typst and optional troff PDF builds;
- repaired and validated EPUB metadata/layout;
- exact-copy delivery to `~/icloud/books`.
- versioned blog `.textpack` bundles delivered to `~/icloud/blogs`.

## Canonical Layout

Each repo should keep book sources under `docs/book/` unless the repo already
has a stronger convention.

```text
docs/book/
  PUBLISH.md
  build.sh
  manuscript.md              # or <repo>.md
  cover.md
  metadata.yaml
  epub.css
  fix_epub_layout.sh
  check_epub_metadata.sh
  render-diagrams.mjs        # or render-diagrams.sh
  diagrams/
    diagram-01.mmd
    diagram-01.png
  dist/
    VERSION.md
    <stem>.pdf
    <stem>.epub
    <stem>.mobi
    <stem> (<version>-<hash>).pdf -> <stem>.pdf
    <stem> (<version>-<hash>).epub -> <stem>.epub
```

For multi-formatter books, use explicit stable stems:

```text
<stem>-typst.{pdf,epub,mobi}
<stem>-troff.{pdf,epub,mobi}
<stem>-typst (<version>-<hash>).{pdf,epub}
<stem>-troff (<version>-<hash>).{pdf,epub}
```

## Artifact Contract

The stable artifact is the byte source. The versioned name is the delivery
surface.

- `metadata.yaml` owns `title`, `subtitle`, `author`, `rights`, and
  `title_stem`.
- The visible reader title stays clean, for example `Typesec` or `LakeCat`.
- The Kindle/catalog title is versioned, for example
  `typesec (0.12.0-a1b2c3)`.
- `dist/VERSION.md` records the exact delivery names.
- Versioned `.epub` and `.pdf` files in `dist/` are generated symlinks or
  aliases and may be ignored by git.
- Delivery to `~/icloud/books` dereferences symlinks so iCloud receives regular
  files with versioned names.

Recommended `VERSION.md` fields for a single-formatter book:

```yaml
kindle_name: typesec (0.12.0-a1b2c3)
version_stamp: 0.12.0-a1b2c3
built_at: 2026-07-05
epub_file: typesec.epub
pdf_file: typesec.pdf
epub_link: typesec (0.12.0-a1b2c3).epub
pdf_link: typesec (0.12.0-a1b2c3).pdf
```

For dual formatter books, suffix the fields:

```yaml
kindle_name_typst: obsidian-typst (0.1.0-a1b2c3)
epub_file_typst: obsidian-typst.epub
pdf_file_typst: obsidian-typst.pdf
epub_link_typst: obsidian-typst (0.1.0-a1b2c3).epub
pdf_link_typst: obsidian-typst (0.1.0-a1b2c3).pdf
kindle_name_troff: obsidian-troff (0.1.0-a1b2c3)
epub_file_troff: obsidian-troff.epub
pdf_file_troff: obsidian-troff.pdf
epub_link_troff: obsidian-troff (0.1.0-a1b2c3).epub
pdf_link_troff: obsidian-troff (0.1.0-a1b2c3).pdf
```

## Mermaid Diagrams

Do not rely on raw Mermaid blocks for final publishing. Render diagrams to
persistent PNG files and rewrite manuscript references before Pandoc/Typst/troff
conversion.

The durable convention is:

- keep `.mmd` sources under `docs/book/diagrams/`;
- keep rendered `.png` files next to the sources;
- reference PNGs from the rendered manuscript;
- copy or mirror the same PNGs into blog asset folders when needed;
- use a white background for blog/Ghost assets and a transparent or white
  background for book assets depending on the book design.

The reusable renderer is:

```sh
node publishing/scripts/render-mermaid.mjs \
  docs/book/manuscript.md \
  /tmp/manuscript.rendered.md \
  docs/book/diagrams
```

## Build Workflow

The reference script is `publishing/scripts/build-book.sh`. Copy it into a repo
or call it with environment variables from a repo-local `docs/book/build.sh`.

Minimal single-formatter build:

```sh
BOOK_ROOT=docs/book \
BOOK_MANUSCRIPT=docs/book/manuscript.md \
BOOK_STEM=typesec \
publishing/scripts/build-book.sh
```

Dual Typst/troff build:

```sh
BOOK_ROOT=docs/book \
BOOK_MANUSCRIPT=docs/book/omnighost.md \
BOOK_FORMATS=typst,troff \
publishing/scripts/build-book.sh
```

The build script:

1. reads the version from Cargo, package.json, `VERSION`, or an override;
2. reads `title_stem` and visible title from `metadata.yaml`;
3. renders Mermaid blocks into persistent `.mmd` and `.png` files;
4. writes `dist/VERSION.md`;
5. renders an unnumbered cover;
6. builds PDF through Typst and, when requested, troff/ms;
7. builds EPUB through Pandoc;
8. runs `fix_epub_layout.sh` when present;
9. creates versioned EPUB/PDF symlinks;
10. runs `check_epub_metadata.sh` when present;
11. builds MOBI when Calibre is available;
12. optionally copies versioned EPUB/PDF artifacts to `~/icloud/books`.

## Cover Rules

Keep cover logic explicit and boring.

- Typst covers should include `#set page(margin: 1in, numbering: none)`.
- Troff covers should live in a raw block such as ```` ```{=ms}````.
- EPUB covers should avoid flexbox and complex viewport sizing.
- Use a placeholder such as `{{KINDLE_NAME}}` for the small version subtitle.
- Build scripts should render the cover into a temporary file, never edit the
  source cover in place.

Fast PDF checks:

```sh
pdftotext -f 1 -l 1 docs/book/dist/<stem>.pdf -
pdftotext -f 2 -l 2 docs/book/dist/<stem>.pdf -
```

Expected result: page 1 extracts cover text without a standalone page number;
page 2 contains the table of contents or body and begins body numbering.

## EPUB Rules

The EPUB must be treated as a packaged artifact, not just a Pandoc output.

- Keep `--epub-title-page=false`.
- Put the custom cover before the nav item in the spine.
- Mark the nav item `linear="no"`.
- Set OPF `dc:title` and title-sort metadata to the versioned
  Kindle/catalog title.
- Keep NCX/nav/visible headings on the clean visible title.
- Reject fallback `UNTITLED` or `Unknown` metadata.
- Keep compact code-block rules in `epub.css`; Pandoc emits one span per source
  line, including empty source lines.

Run the repo-local validator after every build:

```sh
expected_title=$(awk -F': ' '/^kindle_name:/ { print $2 }' docs/book/dist/VERSION.md)
docs/book/check_epub_metadata.sh docs/book/dist/<stem>.epub "$expected_title"
```

## iCloud Delivery

Use `VERSION.md` to resolve destination names. Do not guess from directory
listings, and do not start by listing `~/icloud/books`; macOS/iCloud permissions
can make directory scans fail even when exact file access works.

Preferred verification:

```sh
epub_file=$(awk -F': ' '/^epub_file:/ { print $2 }' docs/book/dist/VERSION.md)
epub_link=$(awk -F': ' '/^epub_link:|^kindle_link:/ { print $2; exit }' docs/book/dist/VERSION.md)
cmp -s "docs/book/dist/$epub_file" "$HOME/icloud/books/$epub_link"
stat "$HOME/icloud/books/$epub_link"
```

Copy with:

```sh
publishing/scripts/publish-versioned-artifacts.sh docs/book/dist
```

The script copies the stable artifact bytes to the versioned destination names
listed in `VERSION.md`.

## Blog Textpacks

Blog posts are centralized around `.textpack` delivery. A textpack is a zipped
TextBundle with Markdown, images, and Omnighost metadata:

```text
docs/blog/<slug>/
  post.md
  diagrams/
    diagram-01.mmd
    diagram-01.png
  dist/
    VERSION.md
    <slug>.textpack
    <slug> (<version>-<hash>).textpack -> <slug>.textpack
```

The zip layout must have the text bundle as the top-level entry:

```text
<slug>.textbundle/
  text.markdown
  info.json
  assets/<image>.png
```

`info.json` carries TextBundle metadata and Omnighost routing metadata:

```json
{
  "version": 2,
  "type": "net.daringfireball.markdown",
  "transient": false,
  "omnighost": {
    "blog": "querygraph.ai",
    "slug": "<slug>",
    "tags": ["querygraph"]
  }
}
```

Build and deliver a versioned blog pack:

```sh
publishing/scripts/publish-versioned-blog.sh docs/blog/<slug>
```

Useful overrides:

```sh
BLOG_DOMAIN=querygraph.ai \
BLOG_TAGS=querygraph,release \
BLOG_EXCERPT="Short Ghost excerpt" \
BLOG_RENDER=1 \
publishing/scripts/publish-versioned-blog.sh docs/blog/<slug>
```

The script:

1. builds `dist/<slug>.textpack`;
2. creates `dist/<slug> (<version>-<hash>).textpack`;
3. writes `dist/VERSION.md`;
4. copies the versioned pack to `~/icloud/blogs`;
5. verifies the copy with `cmp`.

Keep the source post clean for git and the bundled copy clean for editors:

- source post references local images such as `diagrams/diagram-01.png`;
- bundled post rewrites them to `assets/diagram-01.png`;
- source post is not reflowed in place;
- bundled post is reflowed to one line per paragraph for Ulysses/Ghost;
- raw Mermaid blocks are warnings, not final delivery.

## Validation Checklist

Before calling a publishing change done:

- `docs/book/build.sh` or the reference build command succeeds.
- `dist/VERSION.md` names the stable and versioned artifacts.
- Stable PDF and EPUB exist.
- Versioned EPUB/PDF names exist as symlinks or aliases in `dist/`.
- EPUB metadata validation passes.
- PDF page 1/page 2 checks match the cover/body contract.
- Mermaid `.mmd` and `.png` files are materialized for book/blog reuse.
- Blog `.textpack` bundles are built and copied to `~/icloud/blogs` when a
  post is part of the release.
- iCloud delivery, when requested, is verified with exact `cmp` or checksum.
- `git diff --check` is clean.

## Repo Notes

- TypeSec established the clean-title plus versioned-Kindle naming contract.
- LakeCat added stronger persistent Mermaid PNG extraction and automatic
  versioned EPUB/PDF delivery.
- QueryGraph added blog asset mirroring and manuscript/cover render staging.
- OmniGhost added dual Typst/troff PDF formatter output.
- The books archive hardened exact iCloud publishing around `VERSION.md`.
