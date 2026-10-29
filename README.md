# Steamproject Update

Rust CLI for auditing and editing Steamworks store drafts, gameplay stats and
achievements through your own browser session. Includes an interactive terminal and a JSON-lines interface for coding
agents. No Steam password or partner API key is needed by the program.

## Open and run the project (Linux x86_64)

Install Git, Rust/Cargo (stable), Chrome or Chromium, Bash, curl, unzip and
Python 3. You need a Steamworks account with access to the app you want to edit.

Clone the repository and enter its directory:

```bash
git clone git@github.com:moonvent/steamworks_upload.git
cd steamworks_upload
```

SSH cloning requires an SSH key registered with GitHub. Alternatively, use HTTPS:
`git clone https://github.com/moonvent/steamworks_upload.git`.

Open this directory in your editor or continue in the terminal. Install a matching
ChromeDriver and start the utility, replacing `YOUR_APP_ID` with your numeric AppID:

```bash
./tools/install_driver.sh
./run.sh --app YOUR_APP_ID
```

On the first run, Cargo builds the program and the utility creates a private app
configuration. Find your AppID in your Steamworks app list. Keep real AppIDs out
of source files and examples.

### Sign in to Steamworks and open the app page

1. Sign in to Steamworks in the Chrome window and complete Steam Guard. Enter your
   password and verification code directly in the browser.
2. The utility opens the app dashboard at
   `https://partner.steamgames.com/apps/landing/YOUR_APP_ID`.
3. If the editor does not open automatically after sign-in, click **Edit Store Page**
   on the app dashboard. The direct editor address is
   `https://partner.steamgames.com/admin/game/edit/YOUR_APP_ID`.
4. Run `audit` in the utility terminal to check store editor tabs and report gaps.
   Run `fields basic` to list available fields on the Basic Info tab.

The Steamworks store editor requires permission to access the selected app. The
utility uses a dedicated Chrome profile and keeps the session in private storage,
so you may remain signed in on later runs. Use `quit` to close the utility session.

### Resume work and download localization

```bash
cd steamworks_upload
./run.sh --app YOUR_APP_ID
```

In the running utility terminal:

```text
audit
export-localization
```

The export downloads JSON from **Localization → JSON → All Languages** and reports
the file path. Keep translations and working copies in private storage outside the
repository. Empty fields may be absent from the export; export again after preparing
the source text. Localization import is not yet automated.

The `save` command saves a store draft. Publishing is a separate manual step in
Steamworks; the utility does not publish automatically.

Other platforms can build the Rust binary and use a manually installed matching
ChromeDriver; the supplied shell installer and launcher target Linux.

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
- `export-localization`: download the store editor's JSON export with All Languages
  into a new private `apps/<app-id>/localization/<timestamp>/` directory. Validates
  the export controls and downloaded JSON; does not save or publish the store page.
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

## Stats and achievements

Use the existing browser session. These commands open the separate **legacy**
Steamworks settings pages, not the store editor:

```json
{"command":"stats"}
{"command":"achievements"}
{"command":"technical-inspect","section":"stats"}
```

`stats` and `achievements` read the current draft schema through the editor's
reviewed same-origin JSON read routes. `technical-inspect` reports controls in the
reviewed editor container. These commands do not save or publish anything.

Prepare one record at a time. These are synthetic examples, not suggested game
content:

```json
{"command":"stat-plan","definition":{"api_name":"stat_rounds","type":"INT","display_name":"Rounds completed","permission":0,"increment_only":true,"aggregated":false,"default":0,"min":0}}
{"command":"schema-save"}
{"command":"achievement-plan","definition":{"api_name":"ACH_TEN_ROUNDS","display_name":{"english":"Round Trip"},"description":{"english":"Complete ten rounds."},"permission":0,"hidden":false,"progress":{"stat":"stat_rounds","min":0,"max":10}}}
{"command":"schema-save"}
```

The plan reports create/update, current values, desired values and whether anything
changes. It is local and makes no remote edits. A matching API name updates the
existing record; a new name creates a record. Renaming and deleting are unsupported.
`schema-save` applies the pending plan, saves a **Steamworks configuration draft**,
reloads, and reads back persisted values. Publication remains manual and separate
from store-page publication. An unchanged plan does not submit a Save.

Stat definitions replace all applicable settings. Optional `min`, `max`,
`max_change` and `window` are cleared when applicable and omitted/null; `default`, `permission`,
`increment_only` and `aggregated` default to 0, 0, false and false respectively.
Use `INT` for signed 32-bit counters, `FLOAT` for fractional values, or `AVGRATE`
with a positive `window`. Inactive settings for the other stat types are preserved.
AVGRATE does not support increment-only, aggregation or
max-change settings. Existing stat types cannot be changed through this adapter.
Permissions are 0 = Client, 1 = Game Server, 2 = Official Game Server.

Achievement definitions specify permission, visibility and optional progress;
omitting `progress` removes its association. Localized name/description maps update
only supplied languages and preserve existing translations. New achievements need
English names and descriptions. Blank localized text is rejected. A progress stat
must already exist, use the same permission, and admit the requested numeric range.
Stats cannot be changed in ways that invalidate existing linked achievements.
Native localization tokens are preserved or generated from the API name.

Before mutation, the utility compares both schemas with the plan snapshot and
backs up their raw contents plus the target definition under private storage:
`apps/<app-id>/schema-backups/<timestamp>/before.json`. The persisted response is
written to `after.json`. Screenshots and the journal cover these editors too.
Do not edit the same configuration manually while applying a plan.

Steam's native New controls may allocate a draft record immediately. Consequently,
creation happens only inside `schema-save`, and a failed attempt cannot be retried
blindly. Read the draft and private backup first. `schema-discard` drops the local
plan and reloads unsaved inputs after an attempt; it does **not** delete allocated
records or undo saved changes. A new plan starts from the current remote state.
The pending local plan is held only for this utility session.

Store edits and schema plans cannot be pending together. Finish the current plan,
then use `store-open` to return to the store editor. `save` remains the store draft
command; `schema-save` is explicitly for stats/achievements.

Validate a local achievement icon without a browser or account configuration:

```sh
./run.sh validate-achievement-icon /path/to/achievement.png
```

The validator decodes the actual file contents, accepts PNG/JPEG, and rejects
damaged images, non-square images and dimensions below 64x64. It returns JSON on
success and exits unsuccessfully on invalid input. Steam recommends 256x256 PNG;
larger square images are accepted and scaled down by Steam. Other valid sizes
and JPEG produce recommendations rather than errors. Colorful achieved and
grayscale unachieved icons are recommendations, not enforced pixel rules.
See [Steam's achievement icon documentation](https://partner.steamgames.com/doc/features/achievements/ach_guide#icons).
This command only validates a local file; it does not upload, save or publish it.
The achievement upload adapter calls this validator before sending files.

Upload or replace an icon on an existing achievement:

```json
{"command":"achievement-icon-upload","api_name":"ACH_EXAMPLE","file":"~/Pictures/achieved.png","gray":false}
{"command":"achievement-icon-upload","api_name":"ACH_EXAMPLE","file":"~/Pictures/unachieved.png","gray":true}
```

Create the optional stat first, then create/save the achievement with the commands
above, and upload its icons. `gray` selects the unachieved slot; it does not convert
the image to grayscale. Paths support a leading `~/`. Files must pass local
validation and the native 3,000,000-byte limit. No resizing, padding or cropping
happens automatically. Prepare a separate copy if the source needs adjustment.

Each Upload immediately writes that icon to the Steamworks configuration draft;
no additional achievement text Save is needed. The adapter validates the form's
AppID, record IDs, slot, destination and native callback; backs up both schemas
and current icon files privately; uploads a validated private copy; waits for the
native acknowledgement and updated preview; then reloads and verifies the
persisted URL while requiring all other fields, icons, records and stats unchanged.
It also downloads and validates the resulting CDN image. No publication occurs.
After an uncertain upload, inspect the private backup, use `schema-discard` to
reload the editor, and read current schemas before another mutation. Discard does
not undo an upload. Completed earlier uploads remain saved if a later one fails.

Delete a specifically authorized achievement with
`{"command":"achievement-delete","api_name":"ACH_EXAMPLE"}`. This immediately
deletes that definition from the configuration draft through its native Delete
button and exact confirmation dialog. It is not a local plan and is not undone
by `schema-discard`. The utility backs up both schemas, reloads, and requires
the target to be absent while every other achievement and all stats stay unchanged.
If the result is uncertain, read the current schemas and backup before any retry.
Delete a specifically authorized stat with
`{"command":"stat-delete","api_name":"stat_example"}`. The command refuses
stats still linked to achievements. Remove the authorized achievement or its link
first. The native stat Delete control acts immediately without a confirmation
dialog; the utility validates the target row, backs up both schemas and verifies
after reload that only that stat disappeared. Discard cannot undo deletion.
Clearing an individual achievement icon remains unsupported;
the legacy editor allows uploading/replacing icons rather than clearing them.

Achievement **sets, bulk import and publication
are unsupported** by these adapters. Existing icons are preserved and checked.
Supply icons manually in Steamworks for now. A schema record alone does not add
game integration: the game must update stats/unlock achievements via Steamworks.

The legacy page structure and read routes were checked on authenticated pages.
Creation/editing were exercised on a synthetic offline Chrome fixture. Subsequent
authorized creation of a stat and linked achievement passed native draft Save and
readback after reload. Live writes require authorization for specific definitions;
do not perform them merely to test this feature. See Valve's [stats documentation](https://partner.steamgames.com/doc/features/achievements)
and [achievement guide](https://partner.steamgames.com/doc/features/achievements/ach_guide).

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
Stats and achievements have separate commands and are not included in this audit.
Complex wizard state, pricing, other technical app settings, build management and Valve
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

### Reading Tag Wizard data

Open the active app's Tag Wizard manually in the utility-owned browser, then send
`{"command":"tags"}` (or `tags` interactively). The command reads the current
window without navigating, changing selections, saving or publishing. Finish
pending utility edits first. A separate wizard tab must be the WebDriver's current
window; tab switching and tag mutation do not yet have utility commands.

The response separates ordered `developer_tags`, `community_tags`, the embedded
tag `catalog`, and the current step's `choices`. Survey answers are not proof that
those tags made the final prioritized list. This read does not prove public store
visibility. No per-tag Steam descriptions are currently extracted:
`steam_description: null` means unavailable, not a generated definition.
The reader validates the app, native page identity and bootstrap schema, and
returns only tag data rather than scripts, account details or session tokens.
Screenshots and journals remain private. Publication stays manual.
