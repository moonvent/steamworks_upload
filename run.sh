#!/usr/bin/env bash
set -euo pipefail
umask 077
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT"
PRIVATE_ROOT="${STEAMPROJECT_HOME:-${XDG_DATA_HOME:-$HOME/.local/share}/steamproject-update}"
mkdir -p "$PRIVATE_ROOT/runtime"
if [[ "${1:-}" == check || "${1:-}" == init || "${1:-}" == paths ]]; then
    exec cargo run -- "$@"
fi
if [[ "${1:-}" == --help ]]; then
    echo './run.sh --app APP_ID [--json]  # opens Steamworks with private session storage'
    echo './run.sh --headless            # offline browser/adapter check'
    echo './run.sh paths                 # private data locations'
    echo './tools/install_driver.sh      # installs matching ChromeDriver'
    exit 0
fi
DRIVER="${CHROMEDRIVER:-$PRIVATE_ROOT/tools/chromedriver-linux64/chromedriver}"
[[ -x "$DRIVER" ]] || { echo 'Install ChromeDriver: ./tools/install_driver.sh' >&2; exit 1; }
LOG="$(mktemp "$PRIVATE_ROOT/runtime/chromedriver.XXXXXX.log")"
"$DRIVER" --port=0 --allowed-ips=127.0.0.1 > "$LOG" 2>&1 &
DRIVER_PID=$!
trap 'kill "$DRIVER_PID" 2>/dev/null || true' EXIT
PORT=""
for ((attempt=0; attempt<100; attempt++)); do
    if ! kill -0 "$DRIVER_PID" 2>/dev/null; then cat "$LOG" >&2; exit 1; fi
    PORT="$(sed -n 's/.*ChromeDriver was started successfully on port \([0-9][0-9]*\).*/\1/p' "$LOG" | tail -1)"
    [[ -n "$PORT" ]] && break
    sleep 0.1
done
[[ -n "$PORT" ]] || { echo 'ChromeDriver startup timed out.' >&2; exit 1; }
cargo run -- open --webdriver "http://127.0.0.1:$PORT" "$@"
