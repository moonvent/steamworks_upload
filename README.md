# Steamproject Update

Rust CLI for auditing and editing Steamworks store drafts through your own browser
session. Includes an interactive terminal and a JSON-lines interface for coding
agents. No Steam password or partner API key is needed by the program.

## Getting started (Linux x86_64)

Install Rust, Chrome/Chromium, Bash, curl, unzip and Python 3. Then:

```bash
./tools/install_driver.sh
./run.sh --app YOUR_APP_ID
```

Sign in to Steamworks, complete Steam Guard and open **Edit Store Page** if the
editor did not open automatically. The utility uses a dedicated browser profile.
Other platforms can build the Rust binary and use a manually installed matching
ChromeDriver; the supplied shell installer/launcher target Linux.

## Verified page templates and recordings

Page/tab/field recipes live in `templates/store.toml`. Before each read or edit,
checks verify Steamworks origin, AppID, editor/Save endpoint identity, unique form,
tab structure, and the target field's tag/type/name/ID. Tab transitions are awaited.
Unrecognized or changed fields are blocked, rather than located by fuzzy matching.
`fields` reports `template_reviewed` for editable references. Complex widgets need
explicit adapters; their absence is reported instead of guessed.

Checkout templates are reloaded for every operation. Installed binaries include a
fallback copy. `STEAMPROJECT_TEMPLATES=/path/to/store.toml` selects a reviewed custom
recipe file; unsupported schema versions and foreign origins are rejected.

Screenshots and the step journal are enabled by default **only after authenticated
entry to the configured app editor**. Login and Steam Guard screens are skipped.
Each command captures before/after/error screenshots; audits capture every visited
tab. Recordings remain private:

```text
runtime/runs/<run-id>/index.html     # visual timeline
runtime/runs/<run-id>/events.jsonl   # validation checks and operation results
runtime/runs/<run-id>/*.png          # native browser screenshots
runtime/runs/<run-id>/*.mp4          # optional native Chrome video
```

Enable video with `./run.sh --app YOUR_APP_ID --json --video`. It uses experimental
Chrome CDP `Page.startScreenRecording`/`stopScreenRecording`, without an external
recorder. Unsupported Chrome versions are explicitly reported; screenshots remain
available. Video records the page viewport, not other desktop windows, and audio
is disabled. The HTML timeline links screenshots, videos and the step journal.
Never share that private timeline as part of the source package.

## Private storage

Source files contain no real app/account configuration. Private data defaults to:

- Linux: `$XDG_DATA_HOME/steamproject-update` or `~/.local/share/steamproject-update`.
- Other platforms: the OS local application data directory (`dirs::data_local_dir`).

Override the whole private root with `STEAMPROJECT_HOME`. It contains:

```text
apps/<app-id>/store.toml     # local metadata and description files
browser-profile/            # authenticated browser session
runtime/                    # audits, backups, driver logs
tools/                      # downloaded ChromeDriver
active-app                  # last selected app
```

`./run.sh paths` prints the location. Private directories are owner-only on Unix;
the shell launcher uses umask 077. ChromeDriver is restricted to localhost.
An explicit `--config PATH` or `--profile PATH` overrides storage for that item;
keep those paths outside the source tree when sharing it.

## Interactive workflow

- `audit`: visit store-editor tabs and report Steamworks required-section markers,
  asset counters and empty visible fields. Empty optional fields are not blockers.
- `fields basic`: inspect editable controls on a tab. Reports IDs/names, labels,
  field types and select options without dumping values.
- `tab description`: switch tabs.
- `read`: read the base title and descriptions for the configured language.
- `pull`: copy current texts into your private local config, backing up the previous
  config. The description path is in `description_file` (Steam BBCode).
- `plan`: compare local texts against the browser.
- `fill`: fill the base title and descriptions, check readback, do not save.
- `save`: save the pending draft and reload to verify persistence. Never publishes.
- `discard`: reload the editor and drop pending form changes.
- `self-test`: temporary Unicode/BBCode input check, restores fields without Save.
- `quit`: close the session.

Before filling descriptions, use `pull`, edit the private files, then `plan`,
`fill`, `save`. Change `language` for another locale. Localized game titles need
a separate adapter; `name` is the base title. Steam can restrict renaming reviewed
games. JSON requests can also be pasted in the interactive terminal.

## Agent interface

```bash
./run.sh --app YOUR_APP_ID --json
```

Each stdin line is a JSON request. Stdout contains one JSON response per line
plus an initial `ready` event. Progress and diagnostics go to stderr. This makes
it possible for an assistant to audit the game, explain gaps, and fill requested
values using the same browser session, without putting account data in prompts
or repository files.

```json
{"command":"audit"}
{"command":"fields","tab":"basic"}
{"command":"get","tab":"basic","field":"name:app[name]"}
{"command":"set","tab":"basic","field":"name:app[name]","value":"Your Game"}
{"command":"save"}
```

Use references returned by `fields` (e.g. `id:some-control` or `name:app[... ]`).
Supported generic controls: text, textarea, select, checkbox and radio. Pass boolean
values for checkboxes/radios, strings for text/select. Ambiguous names, hidden
fields, password fields, autocomplete widgets and submission buttons are rejected.
For full descriptions, use `pull`/`fill` so PHP and React language state stays in sync.

Additional requests:

```json
{"command":"tab","tab":"description"}
{"command":"pull"}
{"command":"plan"}
{"command":"fill"}
{"command":"discard"}
{"command":"upload","tab":"graphicalassets","field":"id:CONTROL_FROM_FIELDS","file":"/absolute/path/to/image.png"}
{"command":"quit"}
```

File upload is explicit: some widgets send files immediately. Selection is reported
as selection, not as proof of completed upload. Unnamed upload controls and complex
widgets require dedicated adapters. `Publish` is read-only, and publication stays
manual. Standard Save submits the editor form, so avoid simultaneous manual edits
on other tabs. Configuration text patches and generic field patches are separate;
save or discard one before starting the other.

## Template maintenance

A mismatch stops that operation and produces an error snapshot when still on the
authenticated app editor. Inspect the changed page, review the recipe update, then
retry. Do not regenerate templates automatically from a failed page. The template
contains structural metadata only, with no partner account data or real AppIDs.

## Audit coverage

The audit reads all accessible **store-editor** tabs, including publication status
without clicking publish controls. It trusts Steamworks `.required_status` markers
and asset counters instead of guessing that every blank is mandatory. Reports are
private at `runtime/audit.json`. A tab that cannot be read is reported as an error.
Complex wizard state, pricing, technical app settings, build management and Valve
review requirements are not comprehensively validated by this audit. Web UI changes
may require adapter updates. This is an independent tool, not an official Valve SDK.

## Development and distribution

```bash
./run.sh --headless
cargo test
cargo clippy -- -D warnings
cargo package --list --allow-dirty
cargo package --no-verify --allow-dirty
```

The Cargo package uses an explicit allowlist for source, examples, docs and scripts.
It excludes profiles, metadata, reports, driver binaries and build output. The
resulting `.crate` archive is suitable for sharing as source; publishing to crates.io
or GitHub is a separate action. The repository is MIT licensed.

Environment: `STEAMPROJECT_HOME`, `CHROMEDRIVER`, `CHROME_BINARY` (installer).
