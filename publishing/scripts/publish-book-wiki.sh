#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "${REPO_ROOT:-.}" && pwd)"
book_root="${BOOK_ROOT:-docs/book}"
book_dir="$repo_root/$book_root"
dist_dir="${BOOK_DIST_DIR:-$book_dir/dist}"
metadata="${BOOK_METADATA:-$book_dir/metadata.yaml}"
wiki_repo="${BOOK_WIKI_REPO:-}"
wiki_page="${BOOK_WIKI_PAGE:-Book.md}"
wiki_title="${BOOK_WIKI_TITLE:-Book}"
project_name="${BOOK_PROJECT_NAME:-$(basename "$repo_root")}"
public_docs_url="${BOOK_PUBLIC_DOCS_URL:-}"

if [[ -z "$wiki_repo" ]]; then
  remote_url="$(git -C "$repo_root" remote get-url origin)"
  case "$remote_url" in
    git@github.com:*.git)
      wiki_repo="${remote_url%.git}.wiki.git"
      ;;
    https://github.com/*.git)
      wiki_repo="${remote_url%.git}.wiki.git"
      ;;
    *)
      echo "cannot derive GitHub wiki URL from origin: $remote_url" >&2
      exit 2
      ;;
  esac
fi

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

read_marker_value() {
  local key="$1"
  local file="$2"
  awk -F: -v key="$key" '
    $1 == key {
      value = $0
      sub("^[^:]+:[[:space:]]*", "", value)
      print value
      exit
    }
  ' "$file"
}

if [[ ! -f "$metadata" ]]; then
  echo "missing metadata: $metadata" >&2
  exit 2
fi
if [[ ! -f "$dist_dir/VERSION.md" ]]; then
  echo "missing version marker: $dist_dir/VERSION.md" >&2
  exit 2
fi

book_title="${BOOK_VISIBLE_TITLE:-$(read_yaml_value title "$metadata")}"
html_file="$(read_marker_value html_file "$dist_dir/VERSION.md")"
html_link="$(read_marker_value html_link "$dist_dir/VERSION.md")"
epub_file="$(read_marker_value epub_file "$dist_dir/VERSION.md")"
pdf_file="$(read_marker_value pdf_file "$dist_dir/VERSION.md")"
epub_link="$(read_marker_value epub_link "$dist_dir/VERSION.md")"
pdf_link="$(read_marker_value pdf_link "$dist_dir/VERSION.md")"
kindle_link="$(read_marker_value kindle_link "$dist_dir/VERSION.md")"
version_stamp="$(read_marker_value version_stamp "$dist_dir/VERSION.md")"
built_at="$(read_marker_value built_at "$dist_dir/VERSION.md")"

if [[ -z "$epub_link" && -n "$kindle_link" ]]; then
  epub_link="$kindle_link"
fi

tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT
wiki_dir="$tmpdir/wiki"

if ! git clone "$wiki_repo" "$wiki_dir"; then
  echo "Wiki clone failed; initializing a new local wiki checkout for first push: $wiki_repo" >&2
  mkdir -p "$wiki_dir"
  (
    cd "$wiki_dir"
    git init
    git remote add origin "$wiki_repo"
  )
fi

page_path="$wiki_dir/$wiki_page"
mkdir -p "$(dirname "$page_path")"

{
  printf '# %s\n\n' "$wiki_title"
  printf '**%s** ships a generated book as HTML, EPUB, PDF, and MOBI artifacts.\n\n' "$project_name"
  printf '## Current book\n\n'
  printf '%s\n' "- Title: \`$book_title\`"
  printf '%s\n' "- Built: \`${built_at:-unknown}\`"
  printf '%s\n' "- Version stamp: \`${version_stamp:-unknown}\`"
  printf '%s\n' "- Source: \`$book_root\`"
  printf '\n'
  printf '## Local artifacts\n\n'
  [[ -n "$html_file" ]] && printf '%s\n' "- HTML: \`$dist_dir/${html_link:-$html_file}\`"
  [[ -n "$epub_file" ]] && printf '%s\n' "- EPUB: \`$dist_dir/${epub_link:-$epub_file}\`"
  [[ -n "$pdf_file" ]] && printf '%s\n' "- PDF: \`$dist_dir/${pdf_link:-$pdf_file}\`"
  if [[ -n "$html_file" || -n "$epub_file" || -n "$pdf_file" ]]; then
    printf '\n'
  fi
  if [[ -n "$public_docs_url" ]]; then
    printf '## Public documentation\n\n'
    printf '%s\n' "- HTML book: ${public_docs_url%/}/$html_file"
    printf '\n'
  fi
  printf '## Rebuild\n\n'
  printf '```bash\n'
  printf 'cd %s\n' "$repo_root"
  printf '%s/build.sh\n' "$book_root"
  printf '```\n\n'
  printf 'The HTML backend is emitted by FirstPair publishing tooling and is intended to be served directly by the QueryGraph demo documentation page.\n'
} > "$page_path"

(
  cd "$wiki_dir"
  git add "$wiki_page"
  if git diff --cached --quiet; then
    echo "Wiki already current: $wiki_repo $wiki_page"
    exit 0
  fi
  git commit -m "Update book artifact documentation"
  git push origin HEAD
)

echo "Published wiki page: $wiki_repo $wiki_page"
