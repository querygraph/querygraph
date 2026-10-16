"""Refresh only the covers of the pinned, already-published reading edition.

Run with: uv run --project cover python cover/refresh-published-covers.py
The original edition stays pinned so later runs cannot accumulate cover pages.
"""

import copy
import hashlib
import io
import json
from pathlib import Path
import subprocess
import urllib.request
import xml.etree.ElementTree as ET
import zipfile

import pymupdf


ROOT = Path(__file__).resolve().parent.parent
DIST = ROOT / "docs/guide/dist"
BASE = "https://fl6nu3o2c1oqqnum.public.blob.vercel-storage.com/books/querygraph"
INPUTS = {
    "pdf": ("pdf/525f0b6e2c971704-querygraph.pdf", "525f0b6e2c9717049a133067a46752e5b6c27b896963c773fbd1b794d7472755"),
    "epub": ("epub/8a202c7a334a9cd1-querygraph.epub", "8a202c7a334a9cd150425f423c226f8cf8981e89a00ea0ef30c384cba83ce62b"),
}
OPF = "http://www.idpf.org/2007/opf"
DC = "http://purl.org/dc/elements/1.1/"
BOOK_VERSION = "0.4.2"
BOOK_COMMIT = "9ef26f36683b3cf0977b33c2190afbeea8f27141"
ET.register_namespace("", OPF)
ET.register_namespace("dc", DC)


def digest(data):
    return hashlib.sha256(data).hexdigest()


def original(kind):
    name, expected = INPUTS[kind]
    with urllib.request.urlopen(f"{BASE}/{name}", timeout=60) as response:
        data = response.read()
    if digest(data) != expected:
        raise ValueError(f"Original {kind} hash mismatch")
    return data


def refresh_pdf(data, artwork, commit):
    with pymupdf.open(stream=data, filetype="pdf") as old:
        if "covers querygraph (0.4.2-9ef26f3)" not in old[0].get_text():
            raise ValueError("Expected the pinned edition's text cover")
        with pymupdf.open(stream=data, filetype="pdf") as new:
            # Retain the original title-page object for existing destinations,
            # turn it into the information page and prepend the image cover.
            info = new[0]
            info.add_redact_annot(info.rect, fill=(1, 1, 1))
            info.apply_redactions()
            info.insert_textbox(pymupdf.Rect(60, 180, 552, 650),
                "The QueryGraph Stack\n\nAlexy Khrabrov and Slava Tykhonov\n\n"
                f"Book semantic version: {BOOK_VERSION}\nBook source Git commit:\n{BOOK_COMMIT}\n\n"
                f"Cover: v4-landmarks\nCover source Git commit:\n{commit}\n\nFirst Pair Press\nfirstpair.press",
                fontsize=14, align=1)
            page = new.new_page(pno=0, width=612, height=792)
            page.draw_rect(page.rect, color=None, fill=(7 / 255, 29 / 255, 38 / 255))
            page.insert_image(page.rect, stream=artwork, keep_proportion=True)
            new.save(DIST / "querygraph.pdf", garbage=4, deflate=True)
        with pymupdf.open(DIST / "querygraph.pdf") as new:
            assert len(new) == len(old) + 1
            assert old.get_toc() == new.get_toc(), "PDF bookmarks changed"
            assert not new[0].get_text().strip(), "Text cover remains"
            for text in (BOOK_VERSION, BOOK_COMMIT, commit, "firstpair.press"):
                assert text in new[1].get_text(), f"Information page missing {text}"
            images = new[0].get_images()
            assert len(images) == 1, "Expected exactly one cover image"
            embedded = pymupdf.Pixmap(new, images[0][0])
            expected = pymupdf.Pixmap(artwork)
            assert (embedded.width, embedded.height, embedded.samples) == (
                expected.width, expected.height, expected.samples
            ), "Embedded PDF cover pixels differ"
            for index in range(1, len(old)):
                assert old[index].rect == new[index + 1].rect
                assert old[index].get_text() == new[index + 1].get_text()
                assert old[index].get_pixmap().samples == new[index + 1].get_pixmap().samples, f"PDF body page {index + 1} changed"
                before = [{k: v for k, v in link.items() if k != "xref"} for link in old[index].get_links()]
                after = [{k: v for k, v in link.items() if k != "xref"} for link in new[index + 1].get_links()]
                for link in before:
                    if link.get("page", -1) >= 0:
                        link["page"] += 1
                assert before == after, f"PDF links on page {index + 1} changed"
            new[0].get_pixmap(matrix=pymupdf.Matrix(1.5, 1.5)).save(DIST / "cover-proof.png")
            new[1].get_pixmap(matrix=pymupdf.Matrix(1.5, 1.5)).save(DIST / "info-proof.png")
            print(f"PDF: cover pixels verified; all {len(old) - 1} body pages and bookmarks preserved")


def refresh_epub(data, artwork, commit):
    with zipfile.ZipFile(io.BytesIO(data)) as old:
        package = ET.fromstring(old.read("EPUB/content.opf"))
        metadata = package.find(f"{{{OPF}}}metadata")
        manifest = package.find(f"{{{OPF}}}manifest")
        spine = package.find(f"{{{OPF}}}spine")
        # This pinned edition keeps navigation before its text-only title page.
        cover_item = manifest.find(f"{{{OPF}}}item[@href='text/ch001.xhtml']")
        cover_path = "EPUB/" + cover_item.get("href")
        assert b'covers querygraph (0.4.2-9ef26f3)' in old.read(cover_path)
        assert not manifest.findall(f"{{{OPF}}}item[@properties='cover-image']")
        ET.SubElement(metadata, f"{{{OPF}}}meta", {"name": "cover", "content": "v4-cover-image"})
        ET.SubElement(manifest, f"{{{OPF}}}item", {
            "id": "v4-cover-image", "href": "media/querygraph-cover.png",
            "media-type": "image/png", "properties": "cover-image",
        })
        info_ref = spine.find(f"{{{OPF}}}itemref[@idref='{cover_item.get('id')}']")
        spine.remove(info_ref)
        spine.insert(0, info_ref)
        ET.SubElement(manifest, f"{{{OPF}}}item", {"id": "v4-cover-page", "href": "text/cover.xhtml", "media-type": "application/xhtml+xml"})
        spine.insert(0, ET.Element(f"{{{OPF}}}itemref", {"idref": "v4-cover-page"}))
        nav_ref = spine.find(f"{{{OPF}}}itemref[@idref='nav']")
        if nav_ref is not None:
            nav_ref.set("linear", "no")
        width, height = pymupdf.Pixmap(artwork).width, pymupdf.Pixmap(artwork).height
        cover = f'''<?xml version="1.0" encoding="utf-8"?>
<html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops">
<head><title>The QueryGraph Stack</title><style>html,body,#cover-image{{margin:0;padding:0;height:100%;}}svg{{display:block;width:100%;height:100%;}}</style></head>
<body epub:type="cover" id="cover"><div id="cover-image"><svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 {width} {height}" preserveAspectRatio="xMidYMid meet" role="img" aria-label="The QueryGraph Stack cover"><image width="{width}" height="{height}" xlink:href="../media/querygraph-cover.png" /></svg></div></body></html>'''.encode()
        info = f'''<?xml version="1.0" encoding="utf-8"?>
<html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops"><head><title>Edition information</title><style>body{{text-align:center;padding:2em;}}code{{overflow-wrap:anywhere;word-break:break-all;}}</style></head><body epub:type="frontmatter"><section id="querygraph" epub:type="titlepage"><h1>The QueryGraph Stack</h1><p>Alexy Khrabrov and Slava Tykhonov</p><p>Book semantic version: {BOOK_VERSION}</p><p>Book source Git commit:<br/><code>{BOOK_COMMIT}</code></p><p>Cover: v4-landmarks</p><p>Cover source Git commit:<br/><code>{commit}</code></p><p>First Pair Press<br/><a href="https://firstpair.press">firstpair.press</a></p></section></body></html>'''.encode()
        changed = {"EPUB/content.opf": ET.tostring(package, encoding="utf-8", xml_declaration=True), cover_path: info}
        with zipfile.ZipFile(DIST / "querygraph.epub", "w") as new:
            new.writestr("mimetype", old.read("mimetype"), compress_type=zipfile.ZIP_STORED)
            for info in old.infolist():
                if info.filename != "mimetype":
                    new.writestr(copy.copy(info), changed.get(info.filename, old.read(info.filename)))
            new.writestr("EPUB/media/querygraph-cover.png", artwork, compress_type=zipfile.ZIP_DEFLATED)
            new.writestr("EPUB/text/cover.xhtml", cover, compress_type=zipfile.ZIP_DEFLATED)
        with zipfile.ZipFile(DIST / "querygraph.epub") as new:
            assert new.testzip() is None
            assert new.infolist()[0].filename == "mimetype"
            assert new.infolist()[0].compress_type == zipfile.ZIP_STORED
            assert new.read("EPUB/media/querygraph-cover.png") == artwork
            for name in old.namelist():
                if name not in changed:
                    assert old.read(name) == new.read(name), f"EPUB member {name} changed"
            for name in ["META-INF/container.xml", "EPUB/content.opf", cover_path, "EPUB/text/cover.xhtml"]:
                ET.fromstring(new.read(name))
            assert package.find(f"{{{OPF}}}metadata/{{{DC}}}title").text == "querygraph (0.4.2-9ef26f3)"
            assert spine[0].get("idref") == "v4-cover-page"
            assert spine[1].get("idref") == cover_item.get("id")
            for text in (BOOK_VERSION, BOOK_COMMIT, commit, "firstpair.press"):
                assert text.encode() in new.read(cover_path)
            print("EPUB: exact cover bytes, cover-first spine, XML and ZIP verified; all body members preserved")


def main():
    DIST.mkdir(parents=True, exist_ok=True)
    artwork = (ROOT / "cover/querygraph-cover.png").read_bytes()
    commit = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip()
    refresh_pdf(original("pdf"), artwork, commit)
    refresh_epub(original("epub"), artwork, commit)
    name = f"querygraph (0.4.2-9ef26f3-cover-{commit[:7]})"
    fields = {"kindle_name": "querygraph (0.4.2-9ef26f3)", "base_edition": "0.4.2-9ef26f3", "cover_source_commit": commit}
    for kind in INPUTS:
        stable = f"querygraph.{kind}"
        link = f"{name}.{kind}"
        path = DIST / link
        path.unlink(missing_ok=True)
        path.symlink_to(stable)
        fields.update({f"{kind}_file": stable, f"{kind}_link": link, f"{kind}_sha256": digest((DIST / stable).read_bytes())})
    (DIST / "VERSION.md").write_text("".join(f"{key}: {value}\n" for key, value in fields.items()))
    (DIST / "cover-refresh.json").write_text(json.dumps({**fields, "cover_sha256": digest(artwork), "inputs": INPUTS}, indent=2) + "\n")


if __name__ == "__main__":
    main()
