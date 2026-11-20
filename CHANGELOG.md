# Changelog

All notable changes follow [Keep a Changelog](https://keepachangelog.com/) and
[Semantic Versioning](https://semver.org/).

## [0.6.0] — unreleased

### Added
- **Geotechnics, phase 1** (new ribbon group and three panels) — see `docs/GEOTECH.md`:
  - Tables of drillhole logging, face/cell mapping (coordinates or chainage along an axis),
    laboratory tests and oriented discontinuities; column names recognised in es/en/pt, a
    column chooser when one is missing, remembered with the project.
  - **RMR89** (official tables), **Q** and **Q'**, **GSI** (typed, from RMR89' or from Q'),
    RMR ↔ Q correlations (opt-in), RQD from fractures/m (Priest & Hudson), UCS from the
    laboratory by lithology. Missing parameters are reported, never invented.
  - **Hoek-Brown 2002** calculator: mb, s, a, rock mass strength, Mohr-Coulomb fit for tunnels
    or slopes, rock mass modulus (Hoek & Diederichs 2006), envelope chart; GSI and σci / mi
    filled from the classified data and the laboratory.
  - Records drawn on `MIN_GT_SONDAJE` / `MIN_GT_MAPEO` coloured by RMR class, Q class or GSI,
    with XDATA `GT_REC`.
  - **Geotechnical block model**: RMR, GSI and log Q interpolated (IDW / nearest) into the
    resource model (or a new one), rock mass class column, views coloured by them.
  - `MIN_GT_LOAD [logging [mapping [lab [joints]]]]` takes paths as arguments.
- New crate `geotech_engine`. The end-to-end host test also runs the geotechnical module.

## [0.5.0] — 2026-11-20

### Added
- Geological traces from typed **coordinates** (E, N, Z), bearing/azimuth, length and anchor;
  the click mode is now `MIN_GM_TRACE_PICK` (live preview, fills the coordinates).
- **Projection of mapped traces along their dip** (`MIN_GM_PROJECT`): to a target height or by a
  height difference, dip direction to the right/left of the trace or a fixed azimuth, exact
  mitre joins on polylines, closed rings, dip taken from the panel or stored with the trace.

- Several named **contact surfaces** (table) and domains between two surfaces
  (`Debajo C1; Encima C2`); v0.4 single-surface settings still work.
- Composites and the block model are **recovered after reopening** a drawing from
  `<drawing>_composites.csv` / `<drawing>.ombm` (composites only when attribute and
  domains still match).
- **End-to-end CI test against the official Open CAD Studio release** (`host-smoke` job,
  `scripts/smoke_serve.py`).

### Changed
- `MIN_GM_TRACE` now draws from the panel coordinates instead of waiting for a click.
- The cubes view keeps the highest-value blocks when over the limit (it used to keep
  the deepest ones) and reports how many were drawn.
- The composites CSV also stores the from/to coordinates of each composite.
- Dip and its direction are stored in the trace's XDATA (`dip`, `dipdir`).

- `MIN_DH_LOAD [collar [survey [assays [geo]]]]`: file paths as arguments (scripts, automation).

### Fixed (found by the new end-to-end test against the real host)
- Panels were never read directly: the plugin called `panel_snapshot_for_plugin`, which
  out of process always fails; it now uses `panel_snapshot`. Settings typed in a panel no
  longer depend on change notifications arriving before the command runs.
- One unreadable control (a field never edited, an empty dropdown) discarded the whole
  panel; every readable value is now used.

### Removed
- Text annotations of traces (checkbox, `MIN_GM_NOTA` layer, `GM_NOTE` class). The observation
  fields remain as XDATA metadata. Old drawings keep their existing note texts as plain text.

## [0.4.0] — 2026-11-19

### Added
- Resource estimation (phases 1–3): compositing with domains, top-cut and cell
  declustering; domains by geology attribute, 3D solids (tagged with XDATA) and
  a kriged contact surface; experimental variograms (sector method) with model
  editing, automatic fit and live chart; ordinary / simple kriging, IDW and
  nearest neighbour on a rotated regular block model with octant, per-hole and
  multi-pass search; cross-validation; swath plots, grade-tonnage curves and
  resource totals by domain; level, section and cube views; `.ombm` model file
  and CSV export. Three panels in a new **Recursos** ribbon group (es/en/pt).
- Engines `geostat_engine` and `blockmodel_engine`; `solids_engine::inside`
  (point-in-mesh index). 165 k blocks estimated in ≈ 0.4 s.
- See docs/RESOURCES.md.

## [0.3.0] — 2026-11-19

### Added
- Axis designer (`MIN_AX_*`, panel "Ejes"): start point and azimuth typed or
  picked from the drawing (two points, or the end of any polyline);
  straights and curves (radius + length or radius + central angle, right or
  left) with grade in % or degrees; curve discretisation by segment length;
  series mode ("Actualizar punto de inicio") extending one polyline; remove
  last element; axes recoverable from their XDATA.
- Section generator (`MIN_SC_*`, panel "Secciones"): cut meshes (Mesh,
  PolyfaceMesh, 3DFACE) with planes perpendicular to an axis, vertical
  through 2 points with parallels, horizontal levels (automatic range) or
  3 points; 3D section polylines in place and 2D sheets on a grid with
  coordinate grid, title, area and axis marker; replace previous run.
- `solids_engine::{alignment, cut, sheet}` with tests.

 — 2026-11-19

### Added
- Workings module (`solids_engine`, `MIN_UG_*`): rectangular, trapezoidal,
  circular, horseshoe and custom sections; axis and horizontal/vertical
  boundary methods; closed meshes with volume.
- Geological mapping module (`geomap_engine`, `MIN_GM_*`) with live preview.
- Coloured assay/geology traces, interval labels, global or per-hole scale,
  custom ranges, geology colour table.
- XLSX/XLS/ODS import; Windows-1252 decoding; delimiter and decimal-comma
  detection; column-mapping dialog remembered per drawing.
- Settings stored in the drawing (xrecord) and restored on open; per-tab state.
- XDATA schema v1 on every entity, with host display names (es/en/pt).
- Local origin for large UTM coordinates.
- Spanish, English and Portuguese UI.
- `openmining` CLI (`validate`, `desurvey`, `intervals`).
- CI (fmt, clippy, tests on Linux/Windows/macOS), property tests, reference
  data set, benchmarks.

### Changed
- Plugin API 15 → 23 (host release v2026.46); one undo step per action.
- Redraw replaces the previous drawing of the same holes instead of duplicating.
- Entities are created in batches.

### Fixed
- Possible panic on `NaN` depths when sorting.
- See docs/MIGRATION.md for the behaviour fixes relative to the legacy tools.

## [0.1.0] — 2026-11-18

### Added
- Drillhole engine (import, validation, minimum-curvature desurvey, mapping)
  and first plugin skeleton (traces and collar labels).
