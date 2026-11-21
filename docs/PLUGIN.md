# Open Mining — plugin reference

Plugin id `opencad.mining`, command prefix `MIN_`, XDATA application
`OPENMINING`, plugin API 23.

## Commands

Every command below is an operation with `key=value` arguments and a result line for AI
clients; the full list of arguments, results and error codes is in [MCP.md](MCP.md). The
forms below (positional paths, picks) keep working.

| Command | Action |
|---|---|
| `MIN_DH_PANEL` / `MIN_UG_PANEL` / `MIN_GM_PANEL` | Show/hide the module panel |
| `MIN_DH_LOAD [collar [survey [assays [geo]]]]` | Read collar/survey/assays/geology (CSV, TXT, TSV, XLSX, XLS, ODS), map columns, validate. Paths given as arguments (quoted when they contain spaces, `-` to skip one) replace the panel's; scripts and automation use this form |
| `MIN_DH_DRAW` | Draw (replacing any previous drawing of the same holes) |
| `MIN_CH_CONVERT [points [sheet]]` | Convert a channel sampling sheet (topographic point + distance along the working, sample widths, potencia, grades) into drillhole data `CH-…` appended to the loaded drillholes; writes `<drawing>_canales_*.csv`. With *Reconvertir al cargar sondajes* the channels are appended again after every `MIN_DH_LOAD` |
| `MIN_CH_ORIENT` | Orientation of one channel from two clicks (or `p1=`/`p2=`), kept in the orientations table |
| `MIN_DH_CLEAR` | Remove every drillhole entity created by the plugin |
| `MIN_DH_GEOCOLORS` | Fill the geology colour table with the values of the selected attribute |
| `MIN_UG_AXIS` | Solid along a 3D polyline (uses the selection if one entity is selected) |
| `MIN_UG_HBOUND` / `MIN_UG_VBOUND` | Solid from roof/floor/left/right or front/back/left/right |
| `MIN_GM_TRACE` | Draw a trace from the **coordinates** (E, N, Z), **bearing/azimuth**, length and anchor typed in the panel |
| `MIN_GM_TRACE_PICK` | Same, at a clicked point with a live preview; the point fills the coordinate fields |
| `MIN_GM_PROJECT` | Project the selected (or picked) lines/polylines to another height along their dip — see *Geological mapping* |
| `MIN_AX_PANEL` | Show/hide the axis designer |
| `MIN_AX_LINE` / `MIN_AX_CURVE` | Add a straight / curve with the panel values. With **Update start point** the element continues the current axis (same polyline) and the panel start moves to its end; otherwise each element is a new axis from the panel start |
| `MIN_AX_UNDO` | Remove the last element of the current axis |
| `MIN_AX_NEW` | Close the current axis; the next element starts a new one |
| `MIN_AX_PICKSTART` | Pick the start point and a second point for the azimuth |
| `MIN_AX_FROMPOLY` | Pick a polyline: its end point and end azimuth become the start. An Open Mining axis is recovered from its XDATA and continued |
| `MIN_SC_PANEL` | Show/hide the section generator |
| `MIN_SC_RUN` | Cut the selected (or picked) meshes with the configured planes |
| `MIN_AX_SETSTART` / `MIN_AX_SETFROMPOLY` / `MIN_SC_EXEC` | Internal follow-ups of the interactive picks |
| `MIN_UG_RUN` / `MIN_GM_RUN` / `MIN_GM_PROJECT_EXEC` | Internal: follow-ups of the interactive picks |

## Resources

Commands, methods, file format and XDATA of the estimation modules are in
[RESOURCES.md](RESOURCES.md). Second XDATA application `OPENMINING_DOM` marks
solids as estimation domains.

## Geotechnics

Commands, tables, formulas, layers and XDATA (`GT_REC`) of the geotechnical
module are in [GEOTECH.md](GEOTECH.md).

## Geological mapping

**Trace.** E / N / Z + bearing (`N45E`, `S30W`, `N10O`, or a plain azimuth) + length and
anchor (start, centre, end) draw one line on `MIN_GM_TRAZA`. No text is drawn: the optional
*Observación* fields (rock, thickness, mineralisation, alteration, structure, grade, sample),
the dip and its direction are stored in the line's XDATA only.

**Projection along the dip.** Projects traces to another height assuming they lie on a plane
of the given dip (0 < dip ≤ 90, positive down). A vertex at height `z` moves to `z_t` along the
line of steepest descent, i.e. horizontally by

```
s = (z − z_t) / tan(dip)         towards the dip direction when s > 0 (going down),
                                 against it when going up; dip = 90 does not move in plan
```

- *Height*: **Altura de destino** (every vertex goes to that elevation) or **Diferencia de
  altura** (every vertex moves by +up / −down and keeps its own relative height).
- *Dip direction*: to the **right** or **left** of the drawing direction of the trace (so
  the direction in which the line was drawn matters), or a **fixed azimuth/bearing**.
- On a polyline each segment has its own strike; each vertex goes to the intersection of the
  two shifted segments (mitre join, limited to 4× the shift at sharp reversals). Closed
  polylines stay closed.
- *Dip source*: the panel value; if the field is empty, the dip (and dip azimuth) stored with a
  trace drawn by `MIN_GM_TRACE` is used.
- The original is untouched. Each run adds a new line on `MIN_GM_PROYECCION` (colour 30),
  tagged `GM_PROJ`; project several times to build benches.
- Accepts `Line`, `Polyline3D` and `LwPolyline`.

## Axis designer

- Lengths are **horizontal** (plan); Z changes by grade × horizontal length.
  Grade in % or degrees, positive = up, per element.
- Curves: radius + horizontal arc length, or radius + central angle; right =
  clockwise. Drawn as chords no longer than the segment length (per element).
- One `Polyline3D` per axis on `MIN_AX_EJE`; its XDATA stores the start and
  every element, so `MIN_AX_FROMPOLY` can continue it after reopening.

## Sections

| Planes | Input | 2D frame (sheet axes) | Title |
|---|---|---|---|
| Perpendicular to an axis | axis polyline, interval, chainage range (horizontal chainage) | offset (+ right of advance) / elevation | `PK 0+010.00` |
| Vertical through 2 points | 2 points, spacing, count (parallels to the right of p1→p2) | distance from p1 / elevation | `Sección i (desplazamiento d m)` |
| Horizontal levels | elevation from/to/step (to ≤ from = automatic over the solids) | X / Y | `Nivel 4200.00` |
| Through 3 points | 3 points | along p1→p2 / in-plane | `Sección por 3 puntos` |

Solids: `Mesh` (Open Mining solids and any other), `PolyfaceMesh` and `3DFACE`
(all selected 3DFACEs are treated as one surface). ACIS `3DSOLID`s are not
cut. A plane lying exactly on a face (e.g. the gallery end at chainage 0) is
cut a hair inside and projected back. Closed sections report their area.

Sheets are drawn at z = 0 on a grid (sheets per row, gap), to the right of
the solids or at a given X/Y, each with frame, coordinate grid (true
offsets/elevations as labels), title, area and the axis marker. With
**Replace previous sections** the previous run's `SC_*` entities are removed.

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
| `MIN_GM_PROYECCION` | projected traces | 30 |
| `MIN_AX_EJE` | designed axes (3D polylines) | 6 |
| `MIN_SC_CORTE` | 3D sections in place | 1 |
| `MIN_RS_COMP_<VAR>` | composites | per grade |
| `MIN_RS_CONTACTO` | contact surface | 30 |
| `MIN_RS_VISTA` | block model views | per value |
| `MIN_GT_SONDAJE` | classified logging intervals | per class |
| `MIN_GT_MAPEO` | classified face mapping and labels | per class |
| `MIN_ST_ESTEREO` | stereonet drawn in the DWG | 7 |
| `MIN_ST_CUNAS` | wedges on workings | by FS |
| `MIN_SM_GRAFICO` | design charts | 7 |
| `MIN_SC_LAMINA` | sheet frames, titles, areas, sections, markers | 7 |
| `MIN_SC_GRILLA` | sheet grids and grid labels | 8 |

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
| `GM_TRACE` | Traza geológica | `id`, `azimuth`, `length`, `rock`?, `thickness`?, `mineralization`?, `alteration`?, `structure`?, `grade`?, `dip`?, `dipdir`?, `sample`? |
| `GM_PROJ` | Traza proyectada | `src` (hex handle of the source), `dip`, `dipdir`, and `height` (target) or `delta` |
| `AX_AXIS` | Eje de labor | `id`, `e`, `n`, `z`, `az` (start), `length`, `elements`, `e1`…`eN` (see below) |
| `SC_CUT` | Sección 3D | `set` (run id), `plane` (title), `area` |
| `RM_WEDGE` | Cuña | `face`, `mode`, `fs`?, `volume`, `weight`, `sets` |
| `RM_DRAW` | Dibujo geomecánico | `kind`, `series`? |
| `GT_REC` | Registro geotécnico | `src`, `row`, `id`, `rmr`?, `q`?, `qp`?, `gsi`?, `class`?, `lith`?, `label`? — see GEOTECH.md |
| `SC_SHEET` | Lámina de sección | `set`, `plane`, `role` (`frame`/`grid`/`grid_label`/`title`/`area`/`section`/`marker`) |

Axis elements (`eN`), `;`-separated, grade as `p;<percent>` or `d;<degrees>`:
`S;<length>;<grade>;<segment>` and
`C;<radius>;L|A;<length or angle>;R|L;<grade>;<segment>`.

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
