# Architecture

Open Mining follows the three-layer model of the Open CAD Studio plugin
architecture: host (A) ↔ plugin process (B) ↔ headless engines (C).

```
crates/
  mining_core/       C  vectors, azimuth/bearing, LocalOrigin, NaN-safe sorting
  drillhole_engine/  C  table IO (CSV/XLSX, encodings, delimiters) → import (aliases or
                        manual ColumnMap) → validate (R01–R06) → pipeline (desurvey +
                        interval mapping, R07; parallel with rayon) → colour schemes
  solids_engine/     C  sections, AAA frames, axis and boundary methods → ClosedMesh
  geomap_engine/     C  geological trace geometry and annotation layout
  openmining_cli/    C  the engines from the command line (batch, QA, CI)
  opencad_mining/    B  cdylib: manifest, ribbon, panels, commands, XDATA, settings
```

## Rules

1. Engines depend on `std` (+ `mining_core`, optional `rayon`/`calamine`) only:
   no `ocs_plugin_api`, no UI. They are testable, benchmarkable and reusable
   (CLI today, WASM later).
2. Engines return typed errors (`ImportError`, `SolidError`, `BearingError`,
   `MapError`); the plugin translates them (`i18n.rs`). Engines never print.
3. Nothing panics on bad input: NaN-safe sorts (`total_cmp`), explicit
   rejection of non-finite values, no `unwrap` on user data.
4. The plugin never calls `HostApi` from command logic. Commands build
   `Prim`s (geometry + layer + colour + `Tag`) and talk to the `CadSink`
   trait; `HostSink` maps it to `HostApi`, `MockSink` records it in tests.
5. One user action = one undo step (`HostSink` joins nested groups; the
   settings write rides the same step).
6. Domain data lives in the drawing: every entity carries an `OPENMINING`
   XDATA record, the project settings live in an xrecord. A DWG reopened on
   another machine shows the same configuration and its entities can be
   queried, relabelled and redrawn.

## Data flow (drillholes)

```
panel/xrecord ─▶ ProjectConfig ─▶ RawTable::from_path ─▶ import (ColumnMap; dialog on failure)
     ─▶ validate ─▶ DataSet (per tab, in memory)
MIN_DH_DRAW: DataSet ─▶ build_holes (parallel) ─▶ scene() ─▶ Vec<Prim>
     ─▶ HostSink: remove previous entities of the same holes (query_entities by
        xdata_app) ─▶ add_entities in batches of 2 000 ─▶ write_record once (APPID)
```

## State

| Scope | Where | Content |
|---|---|---|
| Drawing | xrecord `OPENMINING/PROJECT` | `ProjectConfig` JSON (files, column maps, options, origin) |
| Drawing | XDATA on entities | identity + attributes (see PLUGIN.md) |
| Tab (session) | `state::TabState` | settings copy + loaded `DataSet`; dropped on `DocumentTabClosed` |
| Process | `state` | last panel values (fallback for hosts without panel snapshots), pending pick |

Interactive commands carry no `HostApi`: a pick stores its result in
`state::Pending` and ends with `CommandStep::Dispatch("MIN_UG_RUN" | "MIN_GM_RUN")`,
whose `dispatch` does the work with full host access.

## Coordinates

Large UTM coordinates lose precision in single-precision viewports. With
**Local origin** enabled, geometry is drawn relative to `(x0, y0, z0)`
(automatic: first collar rounded down to 1 000 m); XDATA keeps world
coordinates.

## Performance

Synthetic project, 1 000 holes × 20 surveys × 200 assays
(`cargo bench -p drillhole_engine`, release build): validate ≈ 45 ms,
desurvey + interval mapping ≈ 80 ms. Entity creation is batched (2 000 per IPC
message) and redraw removal pages through `query_entities` (5 000 per page).
