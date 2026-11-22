# Instructions for AI assistants using Open Mining

Open Mining is a plugin for Open CAD Studio; you drive it with commands through MCP (`ocs_run`).
Start with [docs/AI_QUICKSTART.md](docs/AI_QUICKSTART.md); the reference is [docs/MCP.md](docs/MCP.md)
(`MIN_HELP` gives the same information from the running plugin).

## How to work

1. **Start with `MIN_STATUS`.** It says what is loaded and lists `next` operations with the reason. Call it again
   after any `no_data` error.
2. **Use the typed operations, not guesses.** Arguments are `key=value`; `MIN_HELP op=<name>` gives the exact
   parameters. Do not invent argument names or values: an unknown argument is an error.
3. **Read the result line** (`OPENMINING_RESULT {json}` in the command history) after every command. Act on
   `ok`, `status`, `error.code` / `error.param`, `warnings` and the diagnostics in `data`.
4. **Settings persist.** Arguments that are settings are stored: say what you changed, and do not leave
   experimental values behind without telling the user.
5. **One change at a time.** Change one setting, re-run the stage, compare; do not rebuild everything blindly.
6. **Order matters** for the vein model: data (channels / drillholes) → `vn_surfaces` → `vn_variogram` →
   `vn_estimate` → `vn_cubicate` (or `vn_build`). The `veins` field of `MIN_STATUS` shows how far it is.

## What to verify before giving a number

- Surfaces: `crossings = 0`, `from_intercepts > 0` (or contacts mapped), plausible `plane`, small `rms_*`.
- Estimate: `validation` (cross-validation correlation, composite mean vs prism mean), the `out_*` counts, number of
  `composites`. Few composites or a nugget close to the sill → say the local grades are uncertain.
- Tonnage: each block's `sides`, `dist`, `category` and `why`; `pillars` are reported apart; totals per category.
- Quote the diagnostics together with the number. Say which settings were non-default.

## What not to invent

- **Category rules are the company's norm.** The rules in the table are examples. Do not present their categories
  as a resource classification; ask the user for their rules, set them with `rules=[[…]]`, test with
  `MIN_VN_RULES`, and only then cubicate for a report.
- **Dilution, recovery, density and minimum mining width** are mining decisions: use values the user gave, or
  state clearly that they are defaults (mmw 0.8 m, overbreak 0.10 / 0.05 m, recovery 90 %, density 2.7 t/m³).
- Do not fill in missing data (grades, thickness, geology): the plugin reports them, you report them too.
- Do not claim a validation that was not run, and do not compare with results you have not read.

## Safe use

- Commands that draw replace what the plugin drew before for the same thing and never modify the user's own
  entities. Each command is one undo step (Ctrl+Z).
- Read-only operations (`MIN_STATUS`, `MIN_HELP`, `MIN_ENTITIES`, `MIN_DH_LIST`, `MIN_RS_STATS`, `MIN_BM_SAMPLE`,
  `MIN_BM_RESOURCES`, …) never change the drawing.
- File arguments are opened by the Open CAD Studio process: use paths valid on that machine, quote paths with spaces.
- Save the drawing before asking for exported files (`<drawing>_….csv`): they are written next to it.
- If a command returns `waiting_input`, it started an on-screen pick for an input you did not pass: pass it as an
  argument instead of waiting for the user.

## If you are changing this repository

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) and [docs/TESTING.md](docs/TESTING.md). `docs/MCP.md` is generated:
`OPENMINING_BLESS=1 cargo test -p openmining_ops`. Run `cargo test --workspace` and
`cargo clippy --workspace --all-targets -- -D warnings`. Do not publish, tag or merge without the maintainer's approval.
