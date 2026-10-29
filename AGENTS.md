# Working on Steamproject Update

Use English exclusively in repository files, code, comments, documentation, commit
messages, and GitHub content. Keep account-specific localized store text in private
storage outside the repository.

For session handoff, read `PROJECT_MEMORY.md` first. It points to the private
continuation note outside this repository; keep account-specific context there.

Keep this repository shareable and independent of any Steamworks partner account.
Never put real app IDs, store text, account names, cookies, passwords, profiles,
reports or session identifiers into examples, source code or fixtures.
Private state belongs under `paths::root()` (`STEAMPROJECT_HOME` overrides it).

The supported agent interface is `./run.sh --app APP_ID --json`: send one JSON
request per line. Start with `audit`, then `fields`/`get`; use stable field
references returned by `fields`. `set` fills a field, `save` saves a draft.
Complex React widgets and asset uploads may need dedicated adapters; report
unsupported controls instead of claiming they were updated. Never infer gameplay
facts or contact details from empty fields. Do not add automatic publication.

Before changing store content, make the requested values concrete, preserve a
private backup, verify AppID and language, then fill and verify readback.
Distinguish writing a store draft from publishing it. A field value in the DOM
is not proof that a draft or file upload was saved on Steam.

Validation: `cargo test`, `cargo clippy -- -D warnings`,
`./run.sh --headless` (Chrome/ChromeDriver required), and
`cargo package --list --allow-dirty` when changing distribution contents.
Do not run live saves/uploads just to test code without authorization for those
specific content changes. Read-only audits are fine.

All browser operations must use reviewed templates in `templates/store.toml`.
Validate page identity, element uniqueness and expected semantics before reading or
writing. Wait for Steamworks tab transitions. On real schema drift, stop the
operation and propose a recipe correction; never silently guess another selector.
Keep screenshots/journal enabled for authenticated operations. Video is optional
native Chrome capture (`--video`). Never capture login or Steam Guard pages.

## Handoff procedure

1. Read `PROJECT_MEMORY.md` for architecture, supported operations, validation, and
   known limitations. It contains no account-specific store data.
2. Run `./run.sh paths` to locate private storage. The Linux default is
   `${XDG_DATA_HOME:-$HOME/.local/share}/steamproject-update`; `STEAMPROJECT_HOME`
   overrides it. Read `CONTINUATION_PRIVATE.md` there for the active app, approved
   text, current saved values, outstanding questions, reminders, and artifact paths.
3. Use `active-app` and `apps/<app-id>/store.toml` from private storage. Do not copy
   their values into repository files, fixtures, commit messages, or GitHub content.
4. Start `./run.sh --app APP_ID --json --video` with persistent stdin (TTY when
   needed). Sign-in is manual. Start with `audit`, then `fields`/`get`/`read`.
   Previous reports are snapshots; verify state before changing it.
5. Use `pull` to back up and download current source text before local edits.
   Review `plan`, then `fill`, then `save` only for approved changes. Generic field
   edits use `set` followed by `save`; do not mix them with a pending text patch.
6. `export-localization` downloads JSON for All Languages into a fresh private
   `apps/<app-id>/localization/<timestamp>/` directory. Preserve the original and
   edit a separate working copy. Earlier exports can become stale after edits.
7. Before ending a session, update the private continuation note and save current
   approved content locally. Keep repository handoff instructions in English.
   Close the utility with `quit` once no changes are pending. Do not reuse old
   driver ports/session identifiers, delete live profile locks, or kill unrelated
   browser processes.

## Current workflow and reminders

- The user supplies game details in Russian; prepare English store text and make
  the requested values concrete before entry. Communication can follow the user's
  language; repository files and GitHub content must stay English.
- Remind the user to add translations and GIF animations to the full description
  after the source fields are complete. These are workflow reminders, not scheduled
  notifications. Preserve the private note so another agent can continue them.
- Separate planned features from implemented ones. Controller support must be
  classified from actual menu/setup/gameplay behavior, not a future intention.
- Minimum RAM describes total machine memory, not just game usage. Hardware
  requirements approved for a draft remain provisional until tested on builds.
- A missing Tags marker does not prove that no tags were selected. Inspect the
  Tag Wizard before drawing conclusions. Tag publication is a separate action;
  never publish automatically or assume store Save applies tag changes.

## Implementation notes

- Repository: `https://github.com/moonvent/steamworks_upload`, branch `main`.
  Preserve existing remote history; no force-push is needed for routine updates.
- `src/store.rs` compares descriptions with LF/CRLF equivalence because Steam
  normalizes line endings on Save. Raw snapshot/backup content is preserved, and
  all other content and identity checks remain strict. Do not ignore general
  whitespace or formatting differences to make a save pass.
- Store draft saves and reload verification have been exercised with authorized
  text, contact, and requirement updates. No store page publication was performed.
- Complex Controller Support widgets, platform controls without reviewed recipes,
  Tag Wizard operations, localization import, and asset-specific upload confirmation
  still need dedicated adapters. Do not silently bypass recipes to handle them.
- Screenshots/journals and exports may contain confidential information. Keep them
  private, including handoff artifacts; scan staged files before pushing code.

## Stats and achievements

Use `stats` / `achievements` to read the separate legacy editor schemas. Prepare
one definition with `stat-plan` / `achievement-plan`, review the returned preview,
then use `schema-save` only for approved definitions. Plans are local; native New
controls can allocate server draft records, so creation occurs only during Save.
`schema-discard` does not roll back saved or allocated records. Inspect current
state after failures; never retry creation blindly. `store-open` returns to the
store editor after the schema plan is finished. Do not mix pending store edits and
schema plans. `achievement-delete` with `api_name` immediately deletes one explicitly
authorized achievement through its native control and exact confirmation dialog,
with private backups and verification that all other records and stats are unchanged.
Discard cannot undo deletion. `achievement-icon-upload` uploads/replaces a saved
achievement's icon using api_name, file and gray (false for achieved, true for
unachieved). It validates the image and native form, backs up schemas and existing
images, waits for Upload acknowledgement, and verifies persistence after reload.
Upload immediately saves an image draft; discard cannot undo it. Do not mix it
with pending schema/store edits. Inspect after failures before further writes.
`stat-delete` with api_name immediately deletes an explicitly authorized stat after
checking that no achievement links to it. It backs up both schemas and verifies
that only the named stat disappeared. The native stat deletion has no confirmation
dialog; discard cannot undo it. Remove authorized linked achievements first.
Individual icon clearing, sets, bulk import and publication are not
supported. See README for request fields and complete replacement/patch semantics.

### Reading the full schema for another agent

A new agent with access to this checkout, the private storage and the app's
authenticated browser session can list the current draft stats and achievements.
No Steamworks API key or additional write authorization is needed for these reads.
Manual sign-in may be required if the browser session has expired. Repository
access alone does not grant access to the Steamworks account.

1. Read `PROJECT_MEMORY.md`, run `./run.sh paths`, and read the private
   `CONTINUATION_PRIVATE.md` and `active-app` from that root. Use the actual app
   selected there; never infer an AppID from repository examples.
2. Start `./run.sh --app APP_ID --json --video` with persistent stdin and wait for
   the `ready` event. Replace APP_ID with the private active app value.
3. Send these JSON lines in order, waiting for and checking each response:

   ```json
   {"command":"audit"}
   {"command":"stats"}
   {"command":"achievements"}
   {"command":"quit"}
   ```

4. Successful responses have `ok: true`. Stats are in `result.schema` (an array).
   Achievements are in `result.schema.achievements`; configured achievement
   languages are in `result.schema.languages`. Read the entire returned arrays,
   including hidden achievements and all returned localized text. These are app
   configuration records, not a player's unlocked achievements or stat values.
5. Report exact API names, display names, descriptions, IDs, types, permissions,
   bounds/defaults, increment-only/aggregation settings, visibility, progress-stat
   associations and icon availability when present. Keep fields missing in the
   raw response distinguishable from explicit values. Do not invent data.

`audit` covers store tabs only; it does not replace `stats` or `achievements`.
The latter commands read the legacy Steamworks configuration draft, including
unpublished changes saved with Save. They are not proof of what is published live.
They do not modify definitions, unlock achievements for players or publish.
Advanced achievement sets have no dedicated adapter; do not claim verified set
coverage beyond what the legacy endpoint actually returns. A schema mismatch or
failed response is an error, not an empty list.

The browser requires a single owner. Do not launch concurrent utilities against
the same profile. Reuse a still-running utility only when it belongs to this
ongoing conversation; otherwise close it normally before starting another.
Never copy old driver ports or session identifiers from a handoff note.

For an offline handoff, read the newest applicable private
`apps/<app-id>/schema-snapshots/<timestamp>/schema.json` referenced by the
continuation note. It contains `app_id` and `schemas.stats` /
`schemas.achievements.achievements`. Label snapshots as historical; use a fresh
browser read when the user asks what is currently configured. List commands return
JSON but do not automatically export a snapshot: preserve full responses privately
when asked to record them. Save operations additionally keep private before/after
backups under `apps/<app-id>/schema-backups/`.

### Editing and acceptance status

- Native creation and draft Save of both a stat and linked achievement have been
  exercised with explicitly requested content, including readback after reload.
  Existing manual records were preserved. Exact account-specific definitions,
  backup paths, remaining icons and game-integration work are in the private note.
- For stats, supply the complete desired applicable settings. For achievement
  names/descriptions, supplied language maps patch those languages and preserve
  the others. A new achievement requires English name and description.
- A progress stat is optional. An achievement may be unlocked directly by the
  game; linking a stat is useful for count/progress-based milestones. A linked
  stat must exist first and have compatible permission and numeric bounds.
- The utility configures Steamworks definitions; it does not implement game code.
  Do not modify the separate game project unless requested. Do not claim player
  unlocks work merely because the definition was saved.
- Native text inputs may omit the type attribute: validate the effective DOM
  `type` property. Native stat rows may become `dirty` without a `selected` class;
  identify the one added row by ID and validate its reviewed field semantics.
- If the user edits records manually, wait until they report Save is finished,
  read both schemas, reopen each editor and compare the full responses. Preserve
  their actual values and tokens; do not substitute earlier suggested values.

## Autonomy and confirmations

The user prefers completing requested work without questions about routine steps.
Within the explicitly requested scope, proceed with reads, private backups,
straightforward English translations, technical API names, local edits, required
checks, draft Saves and reload verification without asking again at each step.
Make concrete intended content visible in a brief update before entry. Review
plan previews yourself against the request; showing a preview is not an additional
user approval gate when the requested change is already authorized.

Use the session's existing authorization. An explicit request to create or edit
specific store/stat/achievement content authorizes the necessary draft Save;
do not ask separately whether to save, continue, verify or update handoff notes.
A read-only request authorizes no remote content changes. Infer routine technical
details from established context, but never invent gameplay, contacts, assets or
materially different content to avoid a necessary clarification.

Ask only when a missing decision materially affects the result, instructions
conflict, a new destructive/publication action lacks authorization, or the runtime
requires permission. Publication always needs an explicit user request and remains
unsupported by this utility. Do not treat an earlier creation request as consent
to delete records, publish drafts or make unrelated changes.

These preferences do not change sandbox, GUI, network or filesystem permissions.
Use required escalation tools when those boundaries block an authorized action;
explain that source briefly. Prefer applicable reusable command approvals over
repeated one-off prompts, without asking for unrestricted interpreter rules.

## Reading tags

Use `tags` only with the active app's Tag Wizard already open as the current
WebDriver window. It is read-only and does not navigate, change survey choices,
save or publish. It returns ordered developer tags, community tags, the embedded
catalog and current-step choices separately. Survey choices may differ from the
final prioritized tags. Do not infer public visibility from this read.
Per-tag descriptions are not extracted; null is unavailable data, not a definition.
Separate-tab switching, step navigation and general tag writes remain unsupported.
Keep app-specific tag snapshots and explanations in private storage. The headless
fixture verifies extraction, token filtering, identity and schema rejection.
