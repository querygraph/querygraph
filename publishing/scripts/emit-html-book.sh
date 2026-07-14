#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "${REPO_ROOT:-.}" && pwd)"
book_root="${BOOK_ROOT:-docs/book}"
book_dir="$repo_root/$book_root"
dist_dir="${BOOK_DIST_DIR:-$book_dir/dist}"
build_dir="${BOOK_BUILD_DIR:-$book_dir/build}"
metadata="${BOOK_METADATA:-$book_dir/metadata.yaml}"
cover="${BOOK_HTML_COVER:-${BOOK_COVER_RENDERED:-$book_dir/cover.md}}"
manuscript="${BOOK_HTML_MANUSCRIPT:-${BOOK_RENDERED_MANUSCRIPT:-$book_dir/manuscript.md}}"
css="${BOOK_HTML_CSS:-${BOOK_CSS:-$book_dir/epub.css}}"
resource_path="${BOOK_HTML_RESOURCE_PATH:-$build_dir:$book_dir:$repo_root}"

mkdir -p "$dist_dir" "$build_dir"

read_yaml_value() {
  local key="$1"
  local file="$2"
  awk -F: -v key="$key" '
    $1 ~ "^[[:space:]]*" key "[[:space:]]*$" {
      value = $2
      sub(/^[[:space:]]*/, "", value)
      sub(/[[:space:]]*$/, "", value)
      gsub(/^["'\''"]|["'\''"]$/, "", value)
      print value
      exit
    }
  ' "$file"
}

detect_version() {
  if [[ -n "${BOOK_VERSION:-}" ]]; then
    printf '%s\n' "$BOOK_VERSION"
    return
  fi
  if [[ -f "$repo_root/Cargo.toml" ]]; then
    awk '
      /^\[workspace\.package\]/ { in_workspace_package = 1; next }
      /^\[package\]/ { in_package = 1; next }
      /^\[/ { in_workspace_package = 0; in_package = 0 }
      (in_workspace_package || in_package) && /^version[[:space:]]*=/ {
        gsub(/"/, "", $3)
        print $3
        exit
      }
    ' "$repo_root/Cargo.toml"
    return
  fi
  if [[ -f "$book_dir/VERSION" ]]; then
    tr -d '[:space:]' < "$book_dir/VERSION"
    return
  fi
}

if [[ ! -f "$metadata" ]]; then
  echo "missing metadata: $metadata" >&2
  exit 2
fi
if [[ ! -f "$manuscript" ]]; then
  echo "missing manuscript: $manuscript" >&2
  exit 2
fi

title_stem="${BOOK_STEM:-$(read_yaml_value title_stem "$metadata")}"
visible_title="${BOOK_VISIBLE_TITLE:-$(read_yaml_value title "$metadata")}"
version="$(detect_version)"
git_hash="$(git -C "$repo_root" rev-parse --short HEAD 2>/dev/null || echo nogit)"
pubdate="${BOOK_PUBDATE:-$(date -u +%F)}"

if [[ -z "$title_stem" || -z "$visible_title" ]]; then
  echo "could not read title_stem or title from $metadata" >&2
  exit 1
fi
if [[ -z "$version" ]]; then
  version="0.0.0"
fi

version_stamp="${BOOK_VERSION_STAMP:-$version-$git_hash}"
html_file="$dist_dir/$title_stem.html"
html_link="$dist_dir/$title_stem ($version_stamp).html"
html_title="${BOOK_HTML_TITLE:-$visible_title}"
tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT

sources=()
if [[ -f "$cover" ]]; then
  # Raw PDF-only typst/ms blocks do not belong in the browser edition.
  sed '/^```{=typst}$/,/^```$/d; /^```{=ms}$/,/^```$/d' "$cover" > "$tmpdir/cover.html.md"
  sources+=("$tmpdir/cover.html.md")
fi
sources+=("$manuscript")

pandoc_args=(
  --from markdown+smart
  --standalone
  --toc
  --toc-depth="${BOOK_HTML_TOC_DEPTH:-2}"
  --number-sections
  --metadata-file "$metadata"
  --metadata "title=$html_title"
  --metadata "date=$pubdate"
  --resource-path "$resource_path"
  --output "$html_file"
)

if [[ -f "$css" ]]; then
  pandoc_args+=(--css "$css")
fi
if [[ -n "${BOOK_HTML_LUA_FILTER:-}" ]]; then
  pandoc_args+=(--lua-filter "$BOOK_HTML_LUA_FILTER")
fi

pandoc "${sources[@]}" "${pandoc_args[@]}"

find "$dist_dir" -maxdepth 1 -name "$title_stem (*).html" -exec rm -f {} +
ln -s "$(basename "$html_file")" "$html_link"

if [[ -f "$dist_dir/VERSION.md" ]]; then
  tmp_marker="$tmpdir/VERSION.md"
  awk '
    !/^html_file:/ && !/^html_link:/ && !/^html_title:/ { print }
  ' "$dist_dir/VERSION.md" > "$tmp_marker"
  {
    cat "$tmp_marker"
    printf 'html_file: %s.html\n' "$title_stem"
    printf 'html_link: %s.html\n' "$title_stem ($version_stamp)"
    printf 'html_title: %s\n' "$html_title"
  } > "$dist_dir/VERSION.md"
fi

echo "Built HTML:"
echo "  $html_file"
echo "  $html_link -> $(basename "$html_file")"
