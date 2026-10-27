# Working on Steamproject Update

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
