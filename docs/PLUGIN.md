# Open Mining — plugin reference

Plugin id `opencad.mining`, command prefix `MIN_`, XDATA application
`OPENMINING`, plugin API 23.

## Commands

| Command | Action |
|---|---|
| `MIN_DH_PANEL` / `MIN_UG_PANEL` / `MIN_GM_PANEL` | Show/hide the module panel |
| `MIN_DH_LOAD` | Read collar/survey/assays/geology (CSV, TXT, TSV, XLSX, XLS, ODS), map columns, validate |
| `MIN_DH_DRAW` | Draw (replacing any previous drawing of the same holes) |
| `MIN_DH_CLEAR` | Remove every drillhole entity created by the plugin |
| `MIN_DH_GEOCOLORS` | Fill the geology colour table with the values of the selected attribute |
| `MIN_UG_AXIS` | Solid along a 3D polyline (uses the selection if one entity is selected) |
| `MIN_UG_HBOUND` / `MIN_UG_VBOUND` | Solid from roof/floor/left/right or front/back/left/right |
| `MIN_GM_TRACE` | Geological trace at a picked point (live preview) |
| `MIN_UG_RUN` / `MIN_GM_RUN` | Internal: follow-ups of the interactive picks |

## Import

- Encoding: UTF-8 (with/without BOM) or Windows-1252/Latin-1.
- Delimiter: detected among `,` `;` tab `|`. With `;` (or tab/`|`) numbers use
  a decimal comma (`1.234,5`).
- Headers are matched by alias (see `Field::aliases` in
  `drillhole_engine/src/import.rs`). When a required column is missing, a
  dialog asks for the mapping; it is stored in the drawing and reused.
- Assay values: empty → no value; `<x` → x/2 (R04); text → no value.
- Survey `FROM,TO` layout: a station at FROM and one at TO, same direction.

## Validation rules

| Rule | Behaviour |
|---|---|
| R01 | Survey/assay/geology rows without a collar are dropped (warning) |
| R02 | Survey azimuth normalised to [0, 360); dip clamped to [-90, 90] (warning) |
| R03 | Depth beyond `MaxDepth` raises `MaxDepth` in the cleaned copy (warning); input untouched |
| R05 | Non-finite From → rejected; non-finite To → From + 0.01; From ≥ To → rejected; non-numeric collars/surveys rejected |
| R06 | Per hole, against the furthest accepted To: overlap ≤ 1 mm trims the previous interval; larger → rejected (fatal) |
| R07 | Gaps between intervals become synthetic intervals (no value, drawn grey) for assays **and** geology |

## Layers

| Layer | Content | Colour |
|---|---|---|
| `MIN_DH_TRAZA` | hole traces (3D polylines) | 8 |
| `MIN_DH_COLLAR` | collar labels | 3 |
| `MIN_DH_LEY_<ATTR>` | assay intervals and labels | per value |
| `MIN_DH_GEO_<ATTR>` | geology intervals and labels | per value |
| `MIN_UG_SOLIDO` | solids (Mesh) | 5 |
| `MIN_UG_SECCION` | sections (closed 3D polylines) | 4 |
| `MIN_GM_TRAZA` | geological traces | 1 |
| `MIN_GM_NOTA` | trace annotations | 7 |

## XDATA schema v1 (`OPENMINING`)

Every entity carries one record of `1000` (string) values:

```
OPENMINING-CLASS:<KEY>     identity, read by the host for display names
schema=1
key=value ...              attributes (numbers in shortest round-trip form)
```

Readers must ignore unknown keys; new keys may be added without a schema
bump. A breaking change increments `schema`.

| Class key | Display name (es) | Keys |
|---|---|---|
| `DH_TRACE` | Traza de sondaje | `hole`, `length` |
| `DH_COLLAR` | Collar de sondaje | `hole`, `x`, `y`, `z`, `max_depth` (world coordinates) |
| `DH_ASSAY` | Intervalo de ley | `hole`, `from`, `to`, `attr`, `value`?, `synthetic`? |
| `DH_GEO` | Intervalo geológico | `hole`, `from`, `to`, `attr`, `value`, `lithology` |
| `DH_LABEL` | Etiqueta de sondaje | `hole`, `from`, `to`, `attr` |
| `UG_SOLID` | Sólido de labor | `id`, `method` (`axis`/`boundaries_h`/`boundaries_v`), `profile`, `interval`, `sections`, `volume` |
| `UG_SECTION` | Sección de labor | `solid` (= `UG_SOLID.id`), `station` |
| `GM_TRACE` | Traza geológica | `id`, `azimuth`, `length`, `rock`?, `thickness`?, `mineralization`?, `alteration`?, `structure`?, `grade`?, `dip`?, `sample`? |
| `GM_NOTE` | Anotación geológica | `trace` (= `GM_TRACE.id`) |

Query example (automation API): `{"xdata_app":"OPENMINING","layer":"MIN_DH_*"}`.

## Settings storage

Named-object dictionary `OPENMINING`, xrecord `PROJECT`: the `ProjectConfig`
JSON split into group-code-1 strings of ≤ 1 800 characters. Missing or
unknown fields fall back to defaults, so older and newer plugin versions can
read each other's settings.

## Conventions

- Azimuth 0 = north, clockwise. Drillhole dip is signed (−90 = down).
  Geological-mapping dip is a 0–90 magnitude (positive down).
- Section frame: AAA (no accumulated twist). "Left" alignment uses the
  profile's max u because the frame's right axis points to the physical left
  of the advance direction.
- Solids are closed, outward-oriented meshes (volume reported); no loft
  kernel is required.
