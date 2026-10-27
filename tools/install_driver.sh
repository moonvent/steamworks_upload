#!/usr/bin/env bash
# Install a matching official ChromeDriver in private user data, outside the repo.
set -euo pipefail
umask 077
PRIVATE_ROOT="${STEAMPROJECT_HOME:-${XDG_DATA_HOME:-$HOME/.local/share}/steamproject-update}"
CHROME="${CHROME_BINARY:-}"
if [[ -z "$CHROME" ]]; then CHROME="$(command -v google-chrome || command -v chromium-browser || command -v chromium || true)"; fi
[[ -x "$CHROME" ]] || { echo 'Set CHROME_BINARY to the Chrome executable.' >&2; exit 1; }
case "$(uname -s):$(uname -m)" in
    Linux:x86_64) PLATFORM=linux64 ;;
    *) echo 'This installer currently supports Linux x86_64. Install a matching driver manually and set CHROMEDRIVER.' >&2; exit 1 ;;
esac
VERSION="$("$CHROME" --version)"
BUILD="$(printf '%s\n' "$VERSION" | sed -n 's/.* \([0-9]*\.[0-9]*\.[0-9]*\)\.[0-9]*.*/\1/p')"
[[ -n "$BUILD" ]] || { echo 'Could not determine Chrome version.' >&2; exit 1; }
TMP="$(mktemp -d)"
trap 'rm -rf -- "$TMP"' EXIT
curl --fail --silent --show-error --location 'https://googlechromelabs.github.io/chrome-for-testing/latest-patch-versions-per-build-with-downloads.json' -o "$TMP/versions.json"
URL="$(python3 - "$TMP/versions.json" "$BUILD" "$PLATFORM" <<'PY'
import json,sys
build=json.load(open(sys.argv[1]))['builds'].get(sys.argv[2])
if not build: raise SystemExit('No matching ChromeDriver for this Chrome build')
url=next(x['url'] for x in build['downloads']['chromedriver'] if x['platform']==sys.argv[3])
if not url.startswith('https://storage.googleapis.com/chrome-for-testing-public/'): raise SystemExit('Unexpected driver download origin')
print(url)
PY
)"
curl --fail --location "$URL" -o "$TMP/driver.zip"
unzip -q "$TMP/driver.zip" -d "$TMP/unpacked"
mkdir -p "$PRIVATE_ROOT/tools/chromedriver-linux64"
cp "$TMP/unpacked/chromedriver-linux64/"* "$PRIVATE_ROOT/tools/chromedriver-linux64/"
chmod +x "$PRIVATE_ROOT/tools/chromedriver-linux64/chromedriver"
"$PRIVATE_ROOT/tools/chromedriver-linux64/chromedriver" --version
