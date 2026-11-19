# Changelog

All notable changes follow [Keep a Changelog](https://keepachangelog.com/) and
[Semantic Versioning](https://semver.org/).

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
