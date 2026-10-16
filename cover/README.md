# Published reading-edition cover refresh

The canonical artwork is `querygraph-cover.png`; the original v4 assets and
their generators are archived in `claude/`.

To refresh the covers of the existing public **0.4.2 / 9ef26f3** edition without
changing its body, commit and push the artwork and this builder, then run:

```sh
publishing/scripts/ensure-python-env.sh cover
uv run --project cover python cover/refresh-published-covers.py
```

The builder verifies the pinned original PDF/EPUB hashes, writes the refreshed
artifacts and `VERSION.md` to `docs/guide/dist/`, and checks PDF body rendering,
text, links and bookmarks plus unchanged EPUB body members. Both editions begin
with the v4 image cover and then an information page naming the semantic
version, full book source commit, separate cover source commit, and
`firstpair.press`. The PDF cover and information-page proofs are written beside
the artifacts. `cover-refresh.json` records input and output hashes.

This is a cover refresh of the released reading edition, not a manuscript
rebuild from the current repository version. FirstPair's upload source map may
point PDF/EPUB at these source-owned outputs while retaining its existing HTML
package. Publish with FirstPair's uploader, validate its catalog/site, commit and
push the resulting manifests, deploy production, and verify the stable public
download routes against the source bytes. Deliver regular, versioned PDF/EPUB
copies to iCloud using `VERSION.md`.
