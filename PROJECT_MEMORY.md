# Project memory: Steamproject Update

Updated: 2026-10-27. Read `AGENTS.md` and this file when resuming the project.
Use English exclusively in repository files, source code, comments, documentation,
commit messages, and GitHub content. Account-specific translations belong in
private storage outside the repository.

## Objective and agreed constraints

Build a distributable Rust utility for auditing and editing Steamworks store pages
through a browser. An assistant should inspect tabs, report missing information,
and fill specific user-approved game details. Keep the project independent of any
partner account, game, or workstation.

- Use explicit TOML recipes for pages, tabs, selectors, types, names/IDs, uniqueness,
  language selection, and save verification.
- Validate recipes before reading or editing. On genuine schema drift, stop and
  propose a correction instead of guessing another selector.
- Use visible Chrome, a journal, and before/after/error screenshots.
- Optional native Chrome video starts after sign-in; never capture login or Steam
  Guard pages. Audio is disabled.
- Keep account data, game text, profiles, reports, recordings, and backups outside
  the source tree. Never publish a store page automatically.

## Current state

Rust CLI version 0.3.0, with interactive and JSON Lines interfaces; no GUI.
MIT licensed, with an explicit Cargo package allowlist.

GitHub: `https://github.com/moonvent/steamworks_upload`, branch `main`.
Remote `origin`: `git@github.com:moonvent/steamworks_upload.git`.
The initial GitHub MIT-license commit was merged into local history without
rewriting it. Historical files were checked for private data before pushing.
The project has not been published to crates.io.

Implemented:

- ChromeDriver/Chrome launch with a dedicated persistent browser profile.
- Audits of all nine store editor tabs using Steamworks required-status markers,
  asset counters, and empty-field reports. Optional empty fields are not blockers.
- Read, pull, compare, and fill the base title and descriptions for a selected
  language. Full descriptions use Steam BBCode and the native React plain editor.
- Generic reviewed text, textarea, select, checkbox, and radio controls.
- Draft Save through the native button, followed by reload and readback.
- Private backups, screenshots, JSONL journals, and an HTML operation timeline.
- `export-localization`: native Localization → JSON → All Languages download into
  `apps/<app-id>/localization/<timestamp>/`, with control and JSON validation.
  Verified by a live download without Save. Localization import is not automated.
- Native MP4 via CDP `Page.startScreenRecording`/`stopScreenRecording` and IO stream.
  Unsupported Chrome versions are reported; screenshots remain enabled.
- `templates/store.toml`: page identity, nine tabs, main text fields, 120 additional
  structural recipes, and localization download controls.

Steamworks ignores tab clicks while XFade runs. `browser.rs` waits for effects to
finish before clicking and for the visible active tab afterward. This is a UI
transition delay rather than schema drift.

## Code map

- `src/main.rs`: CLI, JSON protocol, pending changes.
- `src/templates.rs`, `templates/store.toml`: recipe loading and validation.
- `src/browser.rs`: tabs, audits, fields, Save, explicit file selection, exports.
- `src/audit.js`: tab state extraction excluding passwords and hidden fields.
- `src/store.rs`: app config, descriptions, languages, snapshots, backups, fill/save.
- `src/recording.rs`: screenshots, events, HTML timeline, native video.
- `src/paths.rs`: private directories.
- `run.sh`: driver on an available localhost port and CLI startup.
- `tools/install_driver.sh`: matching official ChromeDriver installer, Linux x86_64.
- `README.md`: setup, sign-in, app editor access, workflow, and limitations.

Templates reload before operations. Checkout files take precedence during
development; installed binaries carry an embedded fallback. Set
`STEAMPROJECT_TEMPLATES` for an explicit alternative reviewed TOML file.

## Private state and session handoff

Run `./run.sh paths` to locate private storage. The Linux default is
`${XDG_DATA_HOME:-$HOME/.local/share}/steamproject-update`.
`STEAMPROJECT_HOME` overrides the entire root.

Read `CONTINUATION_PRIVATE.md` under that root for the active game, latest audit,
localization exports, and recording locations. Never copy its contents into the
repository. Private storage contains:

- `active-app`: last selected AppID.
- `apps/<app-id>/store.toml`: game configuration and description file paths.
- `apps/<app-id>/localization/`: exports and translation working copies.
- `browser-profile/`: authenticated browser session; never read or print cookies.
- `runtime/`: reports, backups, screenshots, journals, and videos.
- `tools/`: downloaded ChromeDriver.

Legacy private paths inside the checkout are protected by `.gitignore`. The Cargo
package includes only explicitly listed source, templates, examples, and docs.

## Resuming work

1. Read the private continuation note and identify the active app.
2. Start `./run.sh --app APP_ID --json --video` using the private configuration.
   The user signs in manually if required.
3. Send one JSON request per stdin line. Stdout contains JSON and an initial
   `ready` event; diagnostics go to stderr. Use a persistent stdin/TTY session.
4. Start with a fresh `audit`; previous reports are snapshots.
5. Agree on concrete real text/assets before making the requested changes. Never
   invent contacts, requirements, languages, features, or gameplay facts.
6. Distinguish draft Save from publication. Live saves/uploads require approval for
   those specific content changes; do not perform them merely to test code.

A previous browser may still hold the profile. Do not delete live Chrome lock
files or terminate unrelated Chrome processes. Old session IDs and ports are not
stable; start a new utility session rather than guessing them.

Example requests with placeholder data:

```json
{"command":"audit"}
{"command":"fields","tab":"basic"}
{"command":"read"}
{"command":"pull"}
{"command":"export-localization"}
{"command":"plan"}
{"command":"fill"}
{"command":"get","tab":"basic","field":"name:app[name]"}
{"command":"set","tab":"basic","field":"name:app[name]","value":"Your Game"}
{"command":"save"}
{"command":"discard"}
{"command":"quit"}
```

`pull` backs up and updates the private config and BBCode file. After editing local
text: plan → fill → save. `fill` does not save. Description patches and generic
field patches cannot be mixed before save/discard. Export also requires no pending
changes. Save never publishes the store page.

## Limitations and next improvements

- Complex React wizards, Controller Support, localized game titles, and complex
  upload widgets need dedicated adapters. Do not claim all controls are supported.
- Audits may discover fields without reviewed recipes; generic writes are blocked.
  `fields` reports `template_reviewed`.
- Selecting a file is not proof of successful upload. Some widgets send it
  immediately; asset-specific confirmation is required.
- Audits cover the store editor, not all technical settings, prices, builds, or
  release eligibility requirements.
- Screenshots/video show the page viewport, without other desktop windows or
  sound. Native video is experimental and depends on Chrome support.
- Other operating systems require manual driver installation and binary setup.
- Full live Save acceptance needs a specifically authorized content change.
  Reading, temporary input with restoration, and auditing have been checked.

## Validation history

- `cargo test`: six tests covering request validation, Unicode, description length,
  AppID mismatch, template version, and origin validation.
- `cargo clippy -- -D warnings`.
- `./run.sh --headless --video --profile /tmp/steamproject-browser-smoke`: secret
  filtering, rejection of changed types and ambiguous controls, native MP4.
- Live nine-tab audit and template-based description/contact reading.
- Live all-language localization JSON export without Save or publication.
- `cargo package --list --allow-dirty` and
  `cargo package --no-verify --allow-dirty`: checked for private directories and
  account-specific identifiers. Archive:
  `target/package/steamproject_update-0.3.0.crate`.

Documentation-only handoff updates do not require another browser test. Follow
`AGENTS.md` validation requirements when changing code.
