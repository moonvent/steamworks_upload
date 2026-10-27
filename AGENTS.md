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
