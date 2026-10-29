# Project memory: Steamproject Update

Updated: 2026-10-28. Read `AGENTS.md` and this file when resuming the project.
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
  Authorized text, support-contact, and system-requirement drafts have been saved
  and verified. No store publication was performed.
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
- `src/technical.rs`: stat/achievement definitions, validation, local plans, private
  backups, native legacy editor adapters, save/readback and offline fixture checks.
- `src/achievement_icons.rs`: offline PNG/JPEG decoding and achievement icon
  checks through `./run.sh validate-achievement-icon FILE`; square, minimum 64x64,
  with advisory 256x256 PNG recommendation.
- `src/technical_icons.rs`: native achievement icon upload/replacement, validated
  form identity, private schema/image backups, upload acknowledgement and persisted
  readback after reload. JSON `achievement-icon-upload` uses api_name, file, gray.
- `src/technical_fixture.js`: synthetic offline technical-editor fixture.
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
  Reading, temporary input with restoration, auditing, and authorized draft saves
  have been checked. Do not repeat live writes merely to test code.

Description snapshot equality accepts LF/CRLF differences only, since Steam
normalizes BBCode line endings on Save. Raw strings remain intact in backups.
Identity, actual text changes, blank lines, and trailing newlines remain strict.
`AGENTS.md` contains the operational handoff procedure and current workflow
reminders; account-specific progress remains in the private continuation note.

## Validation history

- `cargo test`: seven tests covering request validation, Unicode, description
  length, AppID mismatch, template version, origin validation, and saved CRLF
  equivalence without hiding real content changes.
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

## Stats and achievements adapters (2026-10-28)

Separate JSON commands: `stats`, `achievements`, `technical-inspect` (section),
`stat-plan` / `achievement-plan` (definition), `schema-save`, `schema-discard`,
and `store-open`. See README for concrete synthetic JSON examples and semantics.
Plans are local, one record at a time, and cannot mix with pending store edits.
Save checks both raw schemas against the plan, backs them up privately, calls the
reviewed native legacy editor, submits that record's Save, reloads and verifies.
New native controls may allocate server draft records immediately; never call them
for read-only tests. Failed mutation attempts block retries until explicit discard
and review of the current draft. Discard does not undo allocated/saved records.

Supported: INT/FLOAT/AVGRATE settings, client/server permissions, localized
achievement names/descriptions, hidden status and existing-stat progress links.
Existing stat type changes are rejected. Missing stat numeric constraints are
cleared; achievement translation maps patch supplied languages and preserve others.
Sets, bulk import and publication have no write adapters.
`stat-delete` with api_name deletes one explicitly authorized stat through its
native Delete control (no confirmation dialog). It rejects achievement-linked
stats, backs up both schemas and verifies that only the target stat disappeared.
Delete the explicitly authorized linked achievement first when removing both.
`achievement-icon-upload` uploads/replaces a single icon on a saved achievement.
It validates square PNG/JPEG files >=64x64 and <=3,000,000 bytes; supports ~/ paths;
backs up schemas and icon files; preserves the native Upload callback; accepts
boolean true or numeric 1 acknowledgement; verifies the persisted CDN URL, image
and all unaffected schema data after reload. Upload saves the image draft directly.
Clearing an individual icon is unsupported. Failed attempts block further writes
until schema-discard and inspection; discard cannot undo uploaded images.
`achievement-delete` now supports explicitly authorized achievement deletion using
the reviewed native Delete control and exact confirmation text. It preserves a
private full-schema backup and verifies after reload that only the target vanished.
It is immediate draft mutation, not a plan; schema-discard cannot undo it.
No game integration is added. The all-store-tabs audit still covers store tabs only.
Templates remain in templates/store.toml, including exact technical paths and
read-only fetch routes. Technical editor screenshots and journal are enabled;
login/Steam Guard screens remain excluded.

Authenticated legacy page structure and read routes were inspected without writes.
Synthetic headless Chrome exercises creation, update, localized values, progress
and rejection of changed field types. Live schema Save/upload is not authorized as
a test and must be validated only with specifically approved content changes.

Validation for these adapters: 15 unit tests, Clippy with warnings denied, headless
Chrome fixture plus native video, and Cargo package list passed. Authenticated
`stats`, `achievements` and `technical-inspect` reads passed on the legacy editors.
The achievements heading lives in #appFrame outside #appInfo; the reviewed recipe
checks that container. No live schema record creation, Save or upload was used for
validation. No publication occurred.

Authorized technical-editor acceptance uncovered two native DOM details: EditStat
sets selected on a jQuery wrapper, while the DOM row becomes dirty through the
type-change handler; new records are now identified by the one added numeric row
ID plus their reviewed API-name control. Native text inputs omit the type
attribute, so adapters validate the effective DOM type property. The offline
fixture now reproduces both details; strict field-type rejection remains enabled.

Authorized native creation of a gameplay stat and a linked achievement has now
completed through schema-save with private before/after backups and successful
persisted-value readback after reload. No publication or icon upload was performed.
The earlier read-only/synthetic-only validation statements describe the initial
implementation checkpoint; this authorized acceptance supersedes that limitation.

AGENTS.md now contains a standalone agent protocol for listing full draft stats
and achievements, JSON result paths, fresh reads versus historical snapshots,
browser ownership, manual-edit verification, and native-editor acceptance details.
It also records the user's preference to avoid routine confirmations: requested
content changes include necessary draft Saves, backups and verification. Runtime
permissions remain separate; no global Codex permission settings were changed.

## Tag Wizard read adapter

The `tags` JSON/interactive command reads the active app's already-open Tag
Wizard in the current WebDriver window. It does not navigate, select, save or
publish. The reviewed recipe is in `templates/store.toml`; `src/tags.rs` and
`src/tags_read.js` validate identity and extract only whitelisted tag data.
Developer order, community weights, catalog and current-step selections are
separate. Steam descriptions are unavailable in this adapter, represented as
null. This is not proof of public visibility. Separate-tab switching, general
step navigation and tag writes remain future work. An isolated headless fixture
checks extraction, private-token exclusion, app mismatch and schema rejection.
