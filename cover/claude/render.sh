#!/bin/zsh
# render.sh <svg> <png> <w> <h> <scale>
svg="$1"; png="$2"; w="$3"; h="$4"; sc="${5:-1}"
html="${svg%.svg}.html"
printf '<!doctype html><html><head><meta charset="utf-8"><style>html,body{margin:0;padding:0;background:#03050C;overflow:hidden}svg{display:block}</style></head><body>' > "$html"
cat "$svg" >> "$html"; printf '</body></html>' >> "$html"
"/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" --headless=new --disable-gpu --hide-scrollbars --no-first-run --no-default-browser-check \
  --user-data-dir="$(dirname "$svg")/.chrome-profile" --force-device-scale-factor="$sc" --window-size="$w,$h" --screenshot="$png" "file://$html" 2>/dev/null
ls -la "$png"
