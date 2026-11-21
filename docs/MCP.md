<!-- Generated from crates/openmining_ops (cargo run -p openmining_cli -- mcp-doc). Do not edit by hand. -->
# Open Mining — operations for AI clients (MCP)

Open CAD Studio exposes the drawing to AI clients with `OpenCADStudio --mcp`. Every Open Mining action below is a command run with the `run` tool (`ocs_run`), with arguments as `key=value` (quote values with spaces: `path="C:\Mis datos\log.csv"`). Arguments marked *setting* are project settings: omitted, the current value is used (the one in the panel); given, it is stored for later runs. Entity inputs that a user would click (axes, solids, points) are passed as handles or coordinates.

Each command writes its usual messages and **one result line** to the command history (read it with the history tool):

```
OPENMINING_RESULT {"op":"gt_load","ok":true,"status":"done","data":{…},"created":["1A3F"],"warnings":[]}
OPENMINING_RESULT {"op":"gt_hb","ok":false,"error":{"code":"out_of_range","message":"…","param":"gsi"}}
```

`status` is `waiting_input` when a pick was started because an input was missing; pass it as an argument instead.

## Error codes

| Code | Meaning |
|---|---|
| `unknown_command` | the command is not an Open Mining operation |
| `unknown_argument` | an argument name is not a parameter of the operation |
| `invalid_argument` | a value has the wrong type or format |
| `out_of_range` | a number is outside its limits |
| `missing_argument` | a required argument was not given |
| `no_data` | something the operation needs is not loaded or computed yet (run the step named in the message) |
| `missing_column` | an input table lacks a required column |
| `not_found` | a referenced entity, file, hole or domain does not exist |
| `needs_input` | the operation waits for a pick; pass the input as an argument instead |
| `cancelled` | the user cancelled a dialog or pick |
| `failed` | the operation could not complete (see the message) |

## Typical workflows

Start with `MIN_STATUS`: it says what is loaded and lists the next operations (`next`). After an error
with code `no_data`, call it again. Entity handles come from the `created` list of a result, from
`MIN_ENTITIES class=...`, or from the host's own query/selection tools. Search the history for the text
`OPENMINING_RESULT ` (the host may put an icon before it) and parse the JSON after it.

**Drillholes**

```
MIN_DH_LOAD collar="C:\data\collar.csv" survey="C:\data\survey.csv" assays="C:\data\assays.csv"
MIN_DH_LIST
MIN_DH_DRAW assay_attr=cu% scheme=rainbow
```

**Channel samples of the workings** (converted into holes `CH-…` appended to the drillholes)

```
MIN_CH_CONVERT points="C:\data\topo_points.csv" sheet="C:\data\channels.csv"
MIN_DH_DRAW assay_attr=au
MIN_CH_ORIENT channel=3950-101-03 p1=0,0,0 p2=1,1,0
```

**Narrow vein surfaces** (hanging wall and footwall from mapped contacts and vein intercepts)

```
MIN_VN_SURFACES vein=V1 hw_layer=VETA_TECHO fw_layer=VETA_PISO vein_attr=tipo vein_values=VETA step=2 radius=40
```

Read `diagnostics` and `worst`: fix the contacts it names, or change `radius`, `step` or
`method`, and run it again.

**Narrow vein prism model** (after the surfaces; edits are polygons drawn on the long section)

```
MIN_VN_VARIOGRAM vein=V1 grade=au thickness_col=potencia lag=10 nlags=10
MIN_VN_ESTIMATE vein=V1 block_u=5 block_v=5 max_distance=40 density=2.7
MIN_VN_LONG vein=V1
MIN_VN_EDIT action=boundary polygon=<handle>
MIN_VN_EDIT action=exclude polygon=<handle>
```

Check `validation` (cross-validation bias and correlation, composites against their
prism) and the `out_*` counts of `summary` before accepting the model.

**Resource estimation** (one variogram model saved per domain before estimating)

```
MIN_RS_COMPOSITE comp_length=2 domains=[["Oxides","OX"],["Sulphides","SU"]]
MIN_RS_STATS
MIN_RS_VARIO domain=Oxides lag=10 nlags=12
MIN_RS_FIT domain=Oxides fit_kind=spherical
MIN_RS_MODELSAVE domain=Oxides
(repeat for every domain)
MIN_RS_ESTIMATE size_x=10 size_y=10 size_z=5 radius1=120 radius2=80 radius3=30
MIN_BM_RESOURCES cutoffs=0.2,0.5,1
MIN_RS_BMVIEW mode=levels attribute=grade_ok
```

Solids as domains: `MIN_RS_SETSOLID target=Oxides solids=1A3F,1A40` (closed meshes).

**Workings and sections**

```
MIN_AX_LINE e=500000 n=8500000 z=3000 bearing=N90E length=100 grade=1
MIN_UG_AXIS axis=<handle of the axis> profile=horseshoe width=4 height=4
MIN_SC_RUN mode=axis axis=<handle> solids=<solid handle> interval=10
```

**Geological mapping**

```
MIN_GM_TRACE e=1000 n=2000 z=300 bearing=N45E length=20 dip=60
MIN_GM_PROJECT traces=<handle> proj_mode=delta proj_value=-10
```

**Geotechnics**

```
MIN_GT_LOAD logging="C:\data\logging.csv" lab="C:\data\lab.csv"
MIN_GT_CLASSIFY
MIN_GT_HB_DATA lithology=Andesite
MIN_GT_ESTIMATE method=idw radius1=60 radius2=60 radius3=20
MIN_BM_SAMPLE from=500000,8500000,3100 to=500100,8500000,3100 columns=gt_rmr,gt_class
MIN_GT_VIEW attribute=class
```

## Project

### `help` — `MIN_HELP`

*Read-only.* Describe the Open Mining operations: the list (name, command, group, summary), or with `op` its parameters, JSON Schema and result; `workflows=true` adds the typical command sequences and the error codes. Use it to discover what can be done without the guide.

| Argument | Type | Setting | Description |
|---|---|---|---|
| `op` | text |  | Operation name or command (gt_view, MIN_GT_VIEW): return its parameters, JSON Schema and result. Without it the list of operations. |
| `group` | text |  | List only one group: project, drillholes, workings, axes, sections, geology, resources, geotechnics, veins. |
| `workflows` | boolean |  | Also return the typical workflows (sequences of commands) and the error codes. |

Result `data`: `groups`, `operations` (list of `{name, command, group, kind, summary}`); with `op`: the same plus `aliases`, `positional`, `parameters` (`{key, type, setting, required, help, options}`), `input_schema`, `result`; with `workflows`: `workflows` (text) and `error_codes` (code -> meaning).

### `status` — `MIN_STATUS`

*Read-only.* What the project holds and what can be done next: drillholes, composites, domains and variogram models, block model, contact surfaces, geotechnical tables, axis being designed, drawing path. Call it first, and after an error with code no_data.

No arguments.

Result `data`: `drawing` (path or null), `drillholes` (`loaded`, `holes`, `assay_attributes`, `geo_attributes`, `assay_attr`), `composites` (`count`, `variable`, `stale`), `domains` (list of `{name, values, density, model_saved}`), `surfaces`, `block_model` (`blocks`, `columns`) or null, `geotechnics` (`tables`, `classified`), `axis` (current axis or null), `next` (suggested operations with the reason).

### `entities` — `MIN_ENTITIES`

*Read-only.* Entities created by Open Mining with their class and attributes (XDATA decoded): volume of solids, area of sections, RMR of geotechnical records, hole/from/to of intervals...

| Argument | Type | Setting | Description |
|---|---|---|---|
| `class` | text |  | Class key filter (DH_TRACE, DH_ASSAY, UG_SOLID, UG_SECTION, GM_TRACE, GM_PROJ, AX_AXIS, SC_CUT, SC_SHEET, RS_COMP, RS_SURFACE, RS_VIEW, GT_REC; comma-separated). |
| `layer` | text |  | Layer filter (exact name). |
| `limit` | integer 1..10000 |  | Maximum entities returned (default 500). |
| `offset` | integer ≥ 0 |  | Entities to skip (paging). |

Result `data`: `total` (matching), `entities`: list of `{handle, class, layer, attributes}`, `next_offset` (null at the end).

## Drillholes

### `dh_list` — `MIN_DH_LIST`

*Read-only.* List the loaded drillholes (collar, depth, interval counts) or, with `hole`, the intervals of one hole with their values.

| Argument | Type | Setting | Description |
|---|---|---|---|
| `hole` | text |  | Hole id: return its intervals (assays and geology) instead of the list of holes. |
| `limit` | integer 1..10000 |  | Maximum holes or intervals returned (default 500). |

Result `data`: `holes`: list of `{id, x, y, z, depth, azimuth, dip, assays, geo}`; with `hole`: `{id, assays: [{from, to, values}], geo: [{from, to, values}]}`.

### `dh_load` — `MIN_DH_LOAD`

Read the drillhole tables (collar required; survey, assays, geology optional; CSV/TXT/TSV/XLSX/XLS/ODS), map their columns (a dialog asks once for columns it cannot recognise; remembered), validate them (rules R01-R07) and keep them for the drawing, composites and geotechnics. Does not draw: run dh_draw.

| Argument | Type | Setting | Description |
|---|---|---|---|
| `collar` | file path | yes | Collar table: hole id, X, Y, Z, length (azimuth/dip optional). |
| `survey` | file path | yes | Survey table: hole id, depth, azimuth, dip (negative down). Empty = straight holes from the collar. |
| `assays` | file path | yes | Assay table: hole id, from, to + numeric attributes. Empty = none. |
| `geo` | file path | yes | Geology table: hole id, from, to + text attributes. Empty = none. |

Positional form: `MIN_DH_LOAD collar survey assays geo` (`-` skips one).

Result `data`: `holes`, `surveys`, `assays`, `geo` (record counts after validation), `assay_attributes`, `geo_attributes`, `errors` and `warnings` (validation messages), `origin` (local drawing origin, `[x,y,z]`).

### `dh_draw` — `MIN_DH_DRAW`

Draw the loaded holes: 3D traces, collars, assay and geology intervals coloured by the chosen attribute, labels. Replaces the previous drawing of the same holes. Needs dh_load.

| Argument | Type | Setting | Description |
|---|---|---|---|
| `draw_trace` | boolean | yes | Draw the hole traces. |
| `draw_collar` | boolean | yes | Draw collar labels. |
| `draw_assays` | boolean | yes | Draw assay intervals coloured by assay_attr. |
| `draw_geo` | boolean | yes | Draw geology intervals coloured by geo_attr. |
| `draw_labels` | boolean | yes | Label the intervals with their value. |
| `label_height` | number ≥ 0.01 | yes | Text height of the labels (drawing units). |
| `assay_attr` | name among the assay attributes | yes | Assay attribute (column) that colours the assay intervals. |
| `scheme` | one of `blue_red`, `green_red`, `rainbow`, `custom` | yes | Colour scale of the assay intervals; custom uses custom_ranges. |
| `scope` | one of `global`, `per_hole` | yes | Range of the colour scale: whole project or each hole. |
| `custom_ranges` | table `Min | Max | Colour` | yes | Colour ranges for the custom scheme: rows of [min, max, colour name or ACI number]. |
| `geo_attr` | name among the geology attributes | yes | Geology attribute (column) that colours the geology intervals. |
| `geo_colors` | table `Value | Colour` | yes | Colour per geology value: rows of [value, colour name or ACI number]. |
| `origin_enabled` | boolean | yes | Draw relative to a local origin (keeps precision with large UTM coordinates). |
| `origin_x` | number | yes | Local origin X (0,0,0 = automatic: first collar rounded to 1000 m). |
| `origin_y` | number | yes | Local origin Y. |
| `origin_z` | number | yes | Local origin Z. |

Result `data`: `holes` (drawn), `entities` (created), `removed` (previous entities replaced).

### `dh_clear` — `MIN_DH_CLEAR`

Remove every drillhole entity created by the plugin.

No arguments.

Result `data`: `removed`.

### `dh_geocolors` — `MIN_DH_GEOCOLORS`

Fill the geology colour table (geo_colors) with the distinct values of geo_attr and default colours. Needs dh_load.

| Argument | Type | Setting | Description |
|---|---|---|---|
| `geo_attr` | name among the geology attributes | yes | Geology attribute whose values get a colour. |

Result `data`: `values` (number of geology values), `colors` (value -> colour).

### `ch_convert` — `MIN_CH_CONVERT`

Convert a channel sampling sheet into drillhole data: each channel is located from a topographic point and a distance along the working (towards the next point or an azimuth), horizontal and perpendicular to the working by default, and becomes a short hole (id prefix CH-) with its samples, thickness (potencia), grades and sample type. The channels are appended to the loaded drillholes (replacing earlier channels), so dh_draw, rs_composite, gt_* and the vein model use them.

| Argument | Type | Setting | Description |
|---|---|---|---|
| `points` | file path | yes | Topographic points table: id, east (X), north (Y), elevation (Z). |
| `sheet` | file path | yes | One row per sample: channel, topographic point, distance from it, next point (or azimuth of the working), sample width (or from/to), vein thickness (potencia), grades; optional: position, height, sample type, working, sample id, channel azimuth / inclination. |
| `prefix` | text | yes | Prefix added to the channel ids to make the hole ids (CH-). |
| `anchor` | one of `centre`, `start` | yes | The channel is centred on the located position, or starts there. |
| `height` | number | yes | Added to the point elevation when the sheet has no height column. |
| `azimuth_offset` | number -360..360 | yes | Default channel azimuth = working direction + this (90 = perpendicular, across the vein when the working follows it). |
| `inclination` | number -90..90 | yes | Default channel inclination, positive up (0 = horizontal). |
| `auto` | boolean | yes | Convert the channels again (and append them) every time the drillholes are loaded. |
| `write_files` | boolean | yes | Write the converted collar / survey / assays / geology CSV next to the drawing. |

Positional form: `MIN_CH_CONVERT points sheet` (`-` skips one).

Result `data`: `channels`, `samples`, `issues` (list of `{channel, row, message}` for skipped channels), `grade_columns`, `text_columns`, `holes` (drillholes + channels loaded), `files` (written CSV or null).

### `ch_orient` — `MIN_CH_ORIENT`

Set the orientation of one channel from two points (picked on screen, or given as p1 and p2): azimuth and inclination of the line p1 -> p2. It is kept in the orientations table and the channels are converted again.

| Argument | Type | Setting | Description |
|---|---|---|---|
| `channel` | text | yes | Channel id (as in the sheet, without prefix) whose orientation is set. |
| `orientations` | table `Channel | Azimuth | Inclination` | yes | Channel orientations that replace the default: rows of [channel, azimuth, inclination]. |
| `p1` | point `x,y[,z]` |  | Start of the channel direction (drawing coordinates). With p1 and p2 no pick is needed. |
| `p2` | point `x,y[,z]` |  | End of the channel direction. |

Positional form: `MIN_CH_ORIENT channel` (`-` skips one).

Result `data`: `channel`, `azimuth`, `inclination`, and the ch_convert result under `converted`.

## Underground workings

### `ug_axis` — `MIN_UG_AXIS`

Closed solid (mesh) of a working along an axis polyline with the configured section.

| Argument | Type | Setting | Description |
|---|---|---|---|
| `profile` | one of `rectangular`, `trapezoidal`, `circular`, `horseshoe`, `custom` | yes | Section shape; custom takes a closed polyline (profile_polyline). |
| `width` | number ≥ 0.01 | yes | Section width (m). |
| `height` | number ≥ 0.01 | yes | Section height (m). |
| `top_width` | number ≥ 0.01 | yes | Top width of a trapezoidal section (m). |
| `radius` | number ≥ 0.01 | yes | Radius of a circular section or of the horseshoe crown (m). |
| `wall_height` | number ≥ 0.01 | yes | Wall height of a horseshoe section (m). |
| `h_align` | one of `left`, `centre`, `right` | yes | Horizontal position of the axis on the section. |
| `v_align` | one of `top`, `centre`, `bottom` | yes | Vertical position of the axis on the section (bottom = axis on the floor). |
| `interval` | number ≥ 0.01 | yes | Distance between sections along the axis (m). |
| `draw_sections` | boolean | yes | Also draw the sections as closed polylines. |
| `tangent_3d` | boolean | yes | Sections perpendicular to the 3D tangent (inclined axes) instead of vertical. |
| `axis` | entity handle (hex) |  | 3D polyline / polyline / line of the axis. Without it: the single selected entity, or a pick (status waiting_input). |
| `profile_polyline` | entity handle (hex) |  | Closed polyline of the custom section (profile = custom), drawn in plan with the axis at 0,0. |

Result `data`: `volume` (m3), `sections`, `solid` (id).

### `ug_hbound` — `MIN_UG_HBOUND`

Closed solid of a working from four boundary polylines: roof, floor, left, right.

| Argument | Type | Setting | Description |
|---|---|---|---|
| `interval` | number ≥ 0.01 | yes | Distance between sections along the axis (m). |
| `draw_sections` | boolean | yes | Also draw the sections as closed polylines. |
| `tangent_3d` | boolean | yes | Sections perpendicular to the 3D tangent (inclined axes) instead of vertical. |
| `lines` | handles `1A,2B` |  | Four polylines in this order: roof, floor, left, right. Without them a pick is started. |

Result `data`: `volume` (m3), `sections`, `solid`.

### `ug_vbound` — `MIN_UG_VBOUND`

Closed solid of a working from four boundary polylines: front, back, left, right.

| Argument | Type | Setting | Description |
|---|---|---|---|
| `interval` | number ≥ 0.01 | yes | Distance between sections along the axis (m). |
| `draw_sections` | boolean | yes | Also draw the sections as closed polylines. |
| `tangent_3d` | boolean | yes | Sections perpendicular to the 3D tangent (inclined axes) instead of vertical. |
| `lines` | handles `1A,2B` |  | Four polylines in this order: front, back, left, right. Without them a pick is started. |

Result `data`: `volume` (m3), `sections`, `solid`.

## Axis designer

### `ax_line` — `MIN_AX_LINE`

Add a straight element (horizontal length + grade) to the axis being designed.

| Argument | Type | Setting | Description |
|---|---|---|---|
| `e` | number | yes | Start point east (world coordinates). |
| `n` | number | yes | Start point north. |
| `z` | number | yes | Start point elevation. |
| `bearing` | text | yes | Start direction: azimuth in degrees or bearing (N45E, S30W, N10O). |
| `grade` | number | yes | Grade of the element (positive up). |
| `grade_unit` | one of `percent`, `degrees` | yes | Unit of the grade. |
| `update_start` | boolean | yes | Continue the current axis: the element is appended and the start moves to its end. Off = each element is a new axis from the start point. |
| `length` | number ≥ 0.001 | yes | Horizontal length of the straight (m). |

Result `data`: `axis` (id), `elements`, `length` (horizontal, m), `end` (`[x,y,z]`), `end_azimuth`.

### `ax_curve` — `MIN_AX_CURVE`

Add a circular curve (radius + length or angle, right/left, with grade) to the axis being designed.

| Argument | Type | Setting | Description |
|---|---|---|---|
| `e` | number | yes | Start point east (world coordinates). |
| `n` | number | yes | Start point north. |
| `z` | number | yes | Start point elevation. |
| `bearing` | text | yes | Start direction: azimuth in degrees or bearing (N45E, S30W, N10O). |
| `grade` | number | yes | Grade of the element (positive up). |
| `grade_unit` | one of `percent`, `degrees` | yes | Unit of the grade. |
| `update_start` | boolean | yes | Continue the current axis: the element is appended and the start moves to its end. Off = each element is a new axis from the start point. |
| `curve_method` | one of `length`, `angle` | yes | Curve given by radius and length, or radius and central angle. |
| `radius` | number ≥ 0.001 | yes | Curve radius (m). |
| `curve_length` | number ≥ 0.001 | yes | Horizontal arc length (m), with curve_method = length. |
| `angle` | number ≥ 0.001 | yes | Central angle (degrees), with curve_method = angle. |
| `turn` | one of `right`, `left` | yes | Turn direction. |
| `segment` | number ≥ 0.001 | yes | Chord length used to draw the arc (m). |

Result `data`: `axis` (id), `elements`, `length` (horizontal, m), `end` (`[x,y,z]`), `end_azimuth`.

### `ax_undo` — `MIN_AX_UNDO`

Remove the last element of the axis being designed.

No arguments.

Result `data`: `axis`, `elements`, `length`, `end`, `end_azimuth` (all null when the axis became empty).

### `ax_new` — `MIN_AX_NEW`

Close the current axis; the next element starts a new one.

No arguments.

Result `data`: empty object.

### `ax_pickstart` — `MIN_AX_PICKSTART`

Set the start point and direction of the axis designer from two points.

| Argument | Type | Setting | Description |
|---|---|---|---|
| `point` | point `x,y[,z]` |  | Start point x,y,z. Without point and toward a pick is started. |
| `toward` | point `x,y[,z]` |  | A second point giving the start azimuth. |

Result `data`: `start` (`[x,y,z]`), `azimuth`.

### `ax_frompoly` — `MIN_AX_FROMPOLY`

Continue from a polyline: its end point and end azimuth become the start. An Open Mining axis is recovered from its XDATA and continued with its elements.

| Argument | Type | Setting | Description |
|---|---|---|---|
| `polyline` | entity handle (hex) |  | Polyline (or Open Mining axis) to continue. Without it a pick is started. |

Result `data`: `start` (`[x,y,z]`), `azimuth`, `restored` (true when an Open Mining axis was recovered), `elements`.

## Sections

### `sc_run` — `MIN_SC_RUN`

Cut solids (meshes) with planes: perpendicular to an axis every interval, vertical through 2 points (+ parallels), horizontal levels, or through 3 points. Draws 3D polylines in place and/or 2D sheets with grid, title and area.

| Argument | Type | Setting | Description |
|---|---|---|---|
| `mode` | one of `axis`, `two_points`, `levels`, `three_points` | yes | How the planes are defined. |
| `interval` | number ≥ 0.001 | yes | Spacing of the planes along the axis (mode axis, m). |
| `ch_from` | number ≥ 0 | yes | First chainage (mode axis, m). |
| `ch_to` | number ≥ 0 | yes | Last chainage (mode axis, m; 0 = axis end). |
| `spacing` | number ≥ 0.001 | yes | Distance between parallel planes (mode two_points, m). |
| `count` | number ≥ 1 | yes | Number of parallel planes (mode two_points). |
| `z_from` | number | yes | Lowest level (mode levels). |
| `z_to` | number | yes | Highest level (mode levels). |
| `z_step` | number ≥ 0.001 | yes | Level spacing (mode levels, m). |
| `draw_3d` | boolean | yes | Draw the sections in place (3D polylines). |
| `draw_sheets` | boolean | yes | Draw 2D sheets (one per plane) with grid, title and area. |
| `columns` | number ≥ 1 | yes | Sheets per row. |
| `gap` | number ≥ 0 | yes | Gap between sheets. |
| `grid` | number ≥ 0.01 | yes | Grid spacing on the sheets (m). |
| `text_height` | number ≥ 0.01 | yes | Text height on the sheets. |
| `auto_place` | boolean | yes | Place the sheets automatically beside the drawing extents. |
| `sheet_x` | number | yes | Sheet insertion X (auto_place off). |
| `sheet_y` | number | yes | Sheet insertion Y (auto_place off). |
| `replace` | boolean | yes | Replace the sections and sheets drawn before. |
| `solids` | handles `1A,2B` |  | Meshes to cut. Without it: the selection, or a pick. |
| `axis` | entity handle (hex) |  | Axis polyline (mode axis). |
| `p1` | point `x,y[,z]` |  | First plane point (modes two_points, three_points). |
| `p2` | point `x,y[,z]` |  | Second plane point. |
| `p3` | point `x,y[,z]` |  | Third plane point (mode three_points). |

Result `data`: `planes`, `cut` (planes that intersect), `polylines`, `sheets`.

## Geological mapping

### `gm_trace` — `MIN_GM_TRACE`

Draw a geological trace (structure, vein, contact) from coordinates, bearing/azimuth and length; dip and observations are stored in its XDATA.

| Argument | Type | Setting | Description |
|---|---|---|---|
| `e` | number | yes | East of the anchor point (world). |
| `n` | number | yes | North of the anchor point. |
| `z` | number | yes | Elevation of the anchor point. |
| `bearing` | text | yes | Azimuth in degrees or bearing (N45E, S30W, N10O). |
| `length` | number ≥ 0.01 | yes | Trace length (m). |
| `anchor` | one of `start`, `centre`, `end` | yes | Which point of the trace the coordinates give. |
| `dip` | text | yes | Dip of the mapped structure in degrees (0-90); stored with the trace. Empty = none. |
| `dip_dir` | one of `right`, `left`, `azimuth` | yes | Dip direction: to the right/left of the drawing direction, or a fixed azimuth (dip_az). |
| `dip_az` | text | yes | Fixed dip direction (azimuth or bearing), with dip_dir = azimuth. |
| `rock_type` | text | yes | Rock type (stored in the trace XDATA). |
| `thickness` | text | yes | Thickness. |
| `mineralization` | text | yes | Mineralization. |
| `alteration` | text | yes | Alteration. |
| `structure` | text | yes | Structure type. |
| `grade` | text | yes | Grade observed. |
| `sample` | text | yes | Sample id. |

Result `data`: `trace` (id), `azimuth`, `bearing`, `start`, `end` (`[x,y,z]`).

### `gm_trace_pick` — `MIN_GM_TRACE_PICK`

Same as gm_trace at a given point (fills e, n, z); without the point a pick with live preview is started.

| Argument | Type | Setting | Description |
|---|---|---|---|
| `at` | point `x,y[,z]` |  | Anchor point x,y,z. Without it a pick is started. |

Result `data`: as gm_trace.

### `gm_project` — `MIN_GM_PROJECT`

Project mapped lines/polylines to another height along their dip (to a target height or by a height difference), down-dip by dz / tan(dip). The dip comes from the arguments/panel, or from the one stored with each trace when dip is empty.

| Argument | Type | Setting | Description |
|---|---|---|---|
| `dip` | text | yes | Dip of the mapped structure in degrees (0-90); stored with the trace. Empty = none. |
| `dip_dir` | one of `right`, `left`, `azimuth` | yes | Dip direction: to the right/left of the drawing direction, or a fixed azimuth (dip_az). |
| `dip_az` | text | yes | Fixed dip direction (azimuth or bearing), with dip_dir = azimuth. |
| `proj_mode` | one of `height`, `delta` | yes | Target height, or height difference (+ up, - down). |
| `proj_value` | number | yes | The target height or the difference (m). |
| `traces` | handles `1A,2B` |  | Lines / polylines to project. Without it: the selection, or a pick. |

Result `data`: `projected` (traces), `skipped` (with reasons), `shift` (horizontal displacement of the first vertex, m).

## Resource estimation

### `rs_stats` — `MIN_RS_STATS`

*Read-only.* Statistics of the current composites by domain (count, mean, declustered mean, CV, quantiles), without recompositing.

No arguments.

Result `data`: `variable`, `domains`: list of `{name, count, length, mean, declustered_mean, cv, min, p25, median, p75, max}`.

### `rs_variogram` — `MIN_RS_VARIOGRAM`

*Read-only.* Experimental variogram points cached for a domain (run rs_vario first) and the working / saved model.

| Argument | Type | Setting | Description |
|---|---|---|---|
| `domain` | name or index among the domains |  | Domain (name or index); default the one of the variography settings. |

Result `data`: `domain`, `directions`: list of `{direction, points: [{lag, gamma, pairs}]}`, `working_model`, `saved_model` (null when not saved).

### `bm_info` — `MIN_BM_INFO`

*Read-only.* Geometry, domains and columns of the block model (tab or .ombm file).

No arguments.

Result `data`: `origin`, `size`, `counts`, `rotation`, `blocks`, `assigned`, `domains` (name -> blocks), `columns` (name -> `{estimated, min, max, mean}`).

### `bm_sample` — `MIN_BM_SAMPLE`

*Read-only.* Values of the block model at a point, or along a line every `step` metres (one entry per block crossed).

| Argument | Type | Setting | Description |
|---|---|---|---|
| `point` | point `x,y[,z]` |  | Block containing this point (world x,y,z). |
| `from` | point `x,y[,z]` |  | Start of a line to sample along (with `to`). |
| `to` | point `x,y[,z]` |  | End of the line. |
| `step` | number ≥ 0.01 |  | Sampling step along the line (default: the smallest block size). |
| `columns` | text |  | Comma-separated columns to return (default all). |

Result `data`: `samples`: list of `{at, block, ijk, domain, values}` (`block` null outside the model).

### `bm_resources` — `MIN_BM_RESOURCES`

*Read-only.* Tonnes, grade, metal and volume by domain above one or more cut-offs on a grade column of the block model.

| Argument | Type | Setting | Description |
|---|---|---|---|
| `column` | text |  | Grade column (default: OK, else NN/IDW/SK of the estimated variable). |
| `cutoffs` | text |  | Comma-separated cut-offs (default: the view cut-off). |

Result `data`: `column`, `table`: list of `{cutoff, rows: [{domain, blocks, tonnes, grade, metal, volume}]}` (a Total row per cut-off).

### `rs_composite` — `MIN_RS_COMPOSITE`

Composite the assays of dh_assay_attr by domain (domains table: geology values, solids, surface conditions), cap them, decluster, report statistics and the histogram. Saves <drawing>_composites.csv. Needs dh_load.

| Argument | Type | Setting | Description |
|---|---|---|---|
| `comp_length` | number ≥ 0.01 | yes | Composite length (m). |
| `comp_remainder` | one of `merge`, `discard`, `keep` | yes | What to do with the short last composite. |
| `comp_min_fraction` | number ≥ 0 | yes | Minimum fraction of the length for a remainder to be kept. |
| `comp_min_coverage` | number ≥ 0 | yes | Minimum sampled fraction of a composite. |
| `unit` | text | yes | Grade unit for reports (%, g/t...). |
| `domains` | table `Domain | Geology values (a;b) | Density t/m³ | Top-cut | Surface` | yes | Estimation domains: rows of [name, geology values a;b, density t/m3, top-cut (0 none), surface condition ('Above C1', 'Below C1; Above C2')]. Empty table = one Global domain. |
| `dc_min` | number ≥ 0.1 | yes | Smallest declustering cell (m). |
| `dc_max` | number ≥ 0.1 | yes | Largest declustering cell (m). |
| `dc_steps` | number ≥ 1 | yes | Number of cell sizes tried. |
| `dc_zratio` | number ≥ 0.01 | yes | Vertical / horizontal cell ratio. |

Result `data`: `variable`, `composites`, `domains` (per domain: `name`, `count`, `mean`, `declustered_mean`, `cv`, `min`, `max`), `file`.

### `rs_drawcomp` — `MIN_RS_DRAWCOMP`

Draw the composites coloured by grade.

No arguments.

Result `data`: `composites` (drawn).

### `rs_setsolid` — `MIN_RS_SETSOLID`

Assign solids (closed meshes) to an estimation domain (stored in their XDATA OPENMINING_DOM).

| Argument | Type | Setting | Description |
|---|---|---|---|
| `target` | name or index among the domains | yes | Domain the solids belong to (name or index). |
| `solids` | handles `1A,2B` |  | Closed meshes. Without it: the selection, or a pick. |

Result `data`: `domain`, `solids` (assigned).

### `rs_contact` — `MIN_RS_CONTACT`

Krige the contact surfaces from the geology contacts in the holes (contacts table) and draw them.

| Argument | Type | Setting | Description |
|---|---|---|---|
| `contacts` | table `Surface | Geology above | Geology below` | yes | Contact surfaces: rows of [name, geology above, geology below]. |
| `step` | number ≥ 0.1 | yes | Grid step of the surfaces (m). |
| `kind` | one of `spherical`, `exponential`, `gaussian` | yes | Variogram type of the surface kriging. |
| `nugget` | number ≥ 0 | yes | Nugget of the surface variogram. |
| `sill` | number ≥ 0 | yes | Sill of the surface variogram. |
| `range` | number ≥ 1 | yes | Range of the surface variogram (m). |
| `radius` | number ≥ 1 | yes | Search radius (m). |
| `min_samples` | number ≥ 1 | yes | Minimum contact points per node. |
| `max_samples` | number ≥ 1 | yes | Maximum contact points per node. |

Result `data`: `surfaces` (per surface: `name`, `points`, `nodes`).

### `rs_vario` — `MIN_RS_VARIO`

Experimental variograms of the domain's composites along the major, semi-major and minor directions (and omnidirectional); shows the chart. Needs rs_composite.

| Argument | Type | Setting | Description |
|---|---|---|---|
| `domain` | name or index among the domains | yes | Estimation domain worked on (name or index). |
| `azimuth` | number | yes | Azimuth of the major direction (experimental and model). |
| `dip` | number | yes | Plunge of the major direction, positive down. |
| `rake` | number | yes | Rotation of the semi-major / minor axes. |
| `lag` | number ≥ 0.1 | yes | Lag distance (m). |
| `nlags` | number ≥ 1 | yes | Number of lags. |
| `az_tol` | number ≥ 0 | yes | Azimuth tolerance (degrees). |
| `dip_tol` | number ≥ 0 | yes | Dip tolerance (degrees). |
| `bw_h` | number ≥ 0 | yes | Horizontal bandwidth (m). |
| `bw_v` | number ≥ 0 | yes | Vertical bandwidth (m). |
| `omni` | boolean | yes | Also compute the omnidirectional variogram. |

Result `data`: `domain`, `pairs` (per direction), `directions`.

### `rs_varioupdate` — `MIN_RS_VARIOUPDATE`

Redraw the variogram chart with the model typed (nugget, two nested structures).

| Argument | Type | Setting | Description |
|---|---|---|---|
| `domain` | name or index among the domains | yes | Estimation domain worked on (name or index). |
| `nugget` | number ≥ 0 | yes | Nugget effect. |
| `s1_kind` | one of `spherical`, `exponential`, `gaussian` | yes | Type of structure 1. |
| `s1_sill` | number ≥ 0 | yes | Sill of structure 1 (0 = unused). |
| `s1_r1` | number ≥ 0.1 | yes | Range of structure 1 along the major direction (m). |
| `s1_r2` | number ≥ 0.1 | yes | Range of structure 1 along the semi-major direction (m). |
| `s1_r3` | number ≥ 0.1 | yes | Range of structure 1 along the minor direction (m). |
| `s2_kind` | one of `spherical`, `exponential`, `gaussian` | yes | Type of structure 2. |
| `s2_sill` | number ≥ 0 | yes | Sill of structure 2 (0 = unused). |
| `s2_r1` | number ≥ 0.1 | yes | Range of structure 2 along the major direction (m). |
| `s2_r2` | number ≥ 0.1 | yes | Range of structure 2 along the semi-major direction (m). |
| `s2_r3` | number ≥ 0.1 | yes | Range of structure 2 along the minor direction (m). |

Result `data`: `domain`, `model` (nugget, structures).

### `rs_fit` — `MIN_RS_FIT`

Fit the variogram model to the experimental variograms (weighted least squares, Nelder-Mead) and put it in the model fields.

| Argument | Type | Setting | Description |
|---|---|---|---|
| `domain` | name or index among the domains | yes | Estimation domain worked on (name or index). |
| `fit_kind` | one of `spherical`, `exponential`, `gaussian` | yes | Structure type to fit. |
| `fit_two` | boolean | yes | Fit two nested structures. |
| `fix_nugget` | boolean | yes | Keep the current nugget. |

Result `data`: `domain`, `model` (nugget, structures), `error`.

### `rs_modelsave` — `MIN_RS_MODELSAVE`

Store the working variogram model as the model of the domain (used by rs_estimate).

| Argument | Type | Setting | Description |
|---|---|---|---|
| `domain` | name or index among the domains | yes | Estimation domain worked on (name or index). |

Result `data`: `domain`.

### `rs_modelload` — `MIN_RS_MODELLOAD`

Load the stored model of the domain into the working model fields.

| Argument | Type | Setting | Description |
|---|---|---|---|
| `domain` | name or index among the domains | yes | Estimation domain worked on (name or index). |

Result `data`: `domain`, `model`.

### `rs_crossval` — `MIN_RS_CROSSVAL`

Leave-one-out cross-validation of the domain's model with the search settings.

| Argument | Type | Setting | Description |
|---|---|---|---|
| `domain` | name or index among the domains | yes | Estimation domain worked on (name or index). |

Result `data`: `domain`, `n`, `mean_error`, `mse`, `correlation`, `slope`.

### `rs_bmcreate` — `MIN_RS_BMCREATE`

Create the block model (geometry settings) and assign each block to its domain (geology, solids, surfaces). Needs rs_composite.

| Argument | Type | Setting | Description |
|---|---|---|---|
| `auto_extent` | boolean | yes | Cover the composites automatically (origin and counts are then computed). |
| `origin_x` | number | yes | Model origin X (corner, auto_extent off). |
| `origin_y` | number | yes | Model origin Y (corner, auto_extent off). |
| `origin_z` | number | yes | Model origin Z (corner, auto_extent off). |
| `count_x` | number ≥ 1 | yes | Blocks along X (auto_extent off). |
| `count_y` | number ≥ 1 | yes | Blocks along Y (auto_extent off). |
| `count_z` | number ≥ 1 | yes | Blocks along Z (auto_extent off). |
| `pad` | number ≥ 0 | yes | Margin added around the composites (auto_extent, m). |
| `size_x` | number ≥ 0.01 | yes | Block size along X (m). |
| `size_y` | number ≥ 0.01 | yes | Block size along Y (m). |
| `size_z` | number ≥ 0.01 | yes | Block size along Z (m). |
| `rotation` | number | yes | Rotation of the model in plan (degrees, clockwise). |
| `disc_x` | number ≥ 1 | yes | Block discretisation along X. |
| `disc_y` | number ≥ 1 | yes | Block discretisation along Y. |
| `disc_z` | number ≥ 1 | yes | Block discretisation along Z. |
| `min_fraction` | number ≥ 0 | yes | Minimum fraction of a block inside its domain. |

Result `data`: `blocks`, `counts` (`[nx,ny,nz]`), `assigned` (blocks in a domain).

### `rs_estimate` — `MIN_RS_ESTIMATE`

Create the block model and estimate each domain with its saved variogram model (OK, SK, IDW, NN; search passes); reports resources by domain at the view cut-off and saves <drawing>.ombm. Needs rs_composite and rs_modelsave for every domain.

| Argument | Type | Setting | Description |
|---|---|---|---|
| `auto_extent` | boolean | yes | Cover the composites automatically (origin and counts are then computed). |
| `origin_x` | number | yes | Model origin X (corner, auto_extent off). |
| `origin_y` | number | yes | Model origin Y (corner, auto_extent off). |
| `origin_z` | number | yes | Model origin Z (corner, auto_extent off). |
| `count_x` | number ≥ 1 | yes | Blocks along X (auto_extent off). |
| `count_y` | number ≥ 1 | yes | Blocks along Y (auto_extent off). |
| `count_z` | number ≥ 1 | yes | Blocks along Z (auto_extent off). |
| `pad` | number ≥ 0 | yes | Margin added around the composites (auto_extent, m). |
| `size_x` | number ≥ 0.01 | yes | Block size along X (m). |
| `size_y` | number ≥ 0.01 | yes | Block size along Y (m). |
| `size_z` | number ≥ 0.01 | yes | Block size along Z (m). |
| `rotation` | number | yes | Rotation of the model in plan (degrees, clockwise). |
| `disc_x` | number ≥ 1 | yes | Block discretisation along X. |
| `disc_y` | number ≥ 1 | yes | Block discretisation along Y. |
| `disc_z` | number ≥ 1 | yes | Block discretisation along Z. |
| `min_fraction` | number ≥ 0 | yes | Minimum fraction of a block inside its domain. |
| `ordinary` | boolean | yes | Ordinary kriging. |
| `simple` | boolean | yes | Simple kriging (declustered mean). |
| `idw` | boolean | yes | Inverse distance. |
| `idw_power` | number ≥ 0.1 | yes | Inverse distance power. |
| `nearest` | boolean | yes | Nearest neighbour. |
| `radius1` | number ≥ 0.1 | yes | Search radius 1 (major, semi-major, minor; oriented like the variogram model, m). |
| `radius2` | number ≥ 0.1 | yes | Search radius 2 (major, semi-major, minor; oriented like the variogram model, m). |
| `radius3` | number ≥ 0.1 | yes | Search radius 3 (major, semi-major, minor; oriented like the variogram model, m). |
| `min_samples` | number ≥ 1 | yes | Minimum composites per block (pass 1). |
| `max_samples` | number ≥ 1 | yes | Maximum composites per block. |
| `max_per_octant` | number ≥ 0 | yes | Maximum composites per octant (0 = no limit). |
| `max_per_hole` | number ≥ 0 | yes | Maximum composites per hole (0 = no limit). |
| `pass2` | number ≥ 0 | yes | Radius multiplier of search pass 2 (<= 1 = no pass). |
| `pass2_min` | number ≥ 1 | yes | Minimum composites of pass 2. |
| `pass3` | number ≥ 0 | yes | Radius multiplier of search pass 3 (<= 1 = no pass). |
| `pass3_min` | number ≥ 1 | yes | Minimum composites of pass 3. |

Result `data`: `blocks`, `estimated`, `seconds`, `domains` (per domain: `name`, `blocks`, `estimated`, `per_pass`, `samples`), `resources` (per domain: `domain`, `tonnes`, `grade`, `blocks`), `saved`.

### `rs_validate` — `MIN_RS_VALIDATE`

Validate the model against the composites: swath plot, grade-tonnage curves or histogram (chart in the panel) and global statistics.

| Argument | Type | Setting | Description |
|---|---|---|---|
| `chart` | one of `swath`, `grade_tonnage`, `histogram` | yes | Chart to show. |
| `swath_axis` | one of `x`, `y`, `z` | yes | Direction of the swath plot. |
| `swath_width` | number ≥ 0.1 | yes | Width of the swaths (m). |

Result `data`: `lines` (validation statistics as text).

### `rs_bmview` — `MIN_RS_BMVIEW`

Draw the block model: level plans, sections through 2 points or along an axis, or cubes; coloured by an attribute (grade, variance, domain, pass, geotechnical columns...).

| Argument | Type | Setting | Description |
|---|---|---|---|
| `mode` | one of `levels`, `two_points`, `axis`, `cubes` | yes | View type. |
| `attribute` | one of `grade_ok`, `ok_variance`, `kriging_efficiency`, `domain`, `pass`, `samples`, `distance`, `nn`, `idw`, `sk`, `tonnage`, `gt_rmr`, `gt_gsi`, `gt_logq`, `gt_class`, `gt_dist`, `gt_n` | yes | Column that colours the blocks. |
| `cutoff` | number | yes | Grade cut-off of the grade attributes. |
| `domain` | name or index among the domains | yes | Domain shown (name or index; 0 = all). |
| `z_from` | number | yes | Lowest level (mode levels; from = to = every block level). |
| `z_to` | number | yes | Highest level (mode levels). |
| `z_step` | number ≥ 0.1 | yes | Level spacing (mode levels). |
| `spacing` | number ≥ 0.1 | yes | Spacing of parallel sections (mode two_points). |
| `count` | number ≥ 1 | yes | Number of parallel sections (mode two_points). |
| `interval` | number ≥ 0.1 | yes | Section spacing along the axis (mode axis). |
| `limit` | number ≥ 1 | yes | Maximum polygons / cubes drawn. |
| `axis` | entity handle (hex) |  | Axis polyline (mode axis). Without it a pick is started. |
| `p1` | point `x,y[,z]` |  | First point of the section line (mode two_points). |
| `p2` | point `x,y[,z]` |  | Second point of the section line. |

Result `data`: `planes`, `polygons`, `limited`, `cubes` (`[drawn,total]` in cubes mode).

### `rs_bmload` — `MIN_RS_BMLOAD`

Load the block model saved beside the drawing (<drawing>.ombm).

| Argument | Type | Setting | Description |
|---|---|---|---|
| `folder` | text | yes | Folder of the model files (empty = beside the drawing). |

Result `data`: `blocks`, `file`.

### `rs_bmexport` — `MIN_RS_BMEXPORT`

Export the block model to <drawing>_bloques.csv.

| Argument | Type | Setting | Description |
|---|---|---|---|
| `folder` | text | yes | Folder of the model files (empty = beside the drawing). |

Result `data`: `file`.

## Geotechnics

### `gt_summary` — `MIN_GT_SUMMARY`

*Read-only.* Summary of the last geotechnical classification (same data as gt_classify, without redrawing).

No arguments.

Result `data`: as gt_classify (`drawn` is null).

### `gt_load` — `MIN_GT_LOAD`

Read the geotechnical tables (logging, face mapping, laboratory, discontinuities; CSV/TXT/TSV/XLSX/XLS/ODS). Columns are recognised by name in es/en/pt; a missing required column opens a column chooser once (remembered). Replaces the tables loaded before.

| Argument | Type | Setting | Description |
|---|---|---|---|
| `logging` | file path | yes | Drillhole logging table by interval (hole, from, to + RQD, UCS, spacing, condition, water, Jn, Jr, Ja, Jw, SRF, ratings...). Located on the holes loaded with dh_load. Empty string = none. |
| `mapping` | file path | yes | Face or cell mapping table: X/Y[/Z] coordinates, or chainage (PK) along a working axis, + the same rating parameters. Empty string = none. |
| `lab` | file path | yes | Laboratory table by lithology: UCS, Is50, mi, Ei, Poisson, density. Supplies UCS when a record lacks it. Empty string = none. |
| `joints` | file path | yes | Oriented discontinuities (dip, dip direction...). Loaded and counted; analysed in a later phase. Empty string = none. |

Positional form: `MIN_GT_LOAD logging mapping lab joints` (`-` skips one).

Result `data`: `tables`: per table (`logging`, `mapping`, `lab`, `joints`) `records`, `columns` (recognised fields), `unused` (headers); `lab_lithologies`; `errors` (messages of tables that could not be read).

### `gt_classify` — `MIN_GT_CLASSIFY`

Classify every logging and mapping record (RMR89, Q, Q', GSI, class I-V), locate it (drillholes from dh_load, mapping by coordinates or chainage) and draw it coloured, replacing the previous drawing. Missing parameters are reported, never invented.

| Argument | Type | Setting | Description |
|---|---|---|---|
| `rmr_version` | one of `rmr89` | yes | RMR system. Only RMR89 (official tables) for now. |
| `correlation` | one of `bieniawski`, `rutledge_preston`, `barton` | yes | Correlation used by cross_fill. |
| `cross_fill` | boolean | yes | When a record has only RMR or only Q, derive the other with the correlation (reported as derived). Off = never invent values. |
| `rqd_from_ff` | boolean | yes | Estimate RQD from the fracture frequency (Priest & Hudson 1976) when RQD is not logged. |
| `color_by` | one of `rmr_class`, `q_class`, `gsi` | yes | Colour of the drawn records. |
| `labels` | boolean | yes | Draw an 'RMR · Q · GSI' label at each mapped record. |
| `label_height` | number ≥ 0.01 | yes | Text height of the labels (drawing units). |
| `axes` | handles `1A,2B` |  | Axis polylines of the workings, for mapping records located by chainage. With several axes the table's Axis column is matched to each polyline's layer. Without it a pick is started (status waiting_input). |

Result `data`: `records`, `with_rmr`, `with_q`, `with_gsi`, `unclassified`, `classes` ([I..V] counts), `missing` (parameter -> records), `invalid` (list of `{table,row,field,value}`), `derived` (kind -> records), `unlocated` (count) and `unlocated_examples`, `drawn`.

### `gt_clear` — `MIN_GT_CLEAR`

Remove the drawn geotechnical records.

No arguments.

Result `data`: `removed`.

### `gt_hb` — `MIN_GT_HB`

Generalised Hoek-Brown 2002: mb, s, a, rock mass strengths, equivalent Mohr-Coulomb c' and phi' (tunnel or slope range) and rock mass modulus (Hoek-Diederichs 2006); shows the envelope chart in the panel.

| Argument | Type | Setting | Description |
|---|---|---|---|
| `sigci` | number ≥ 0.1 | yes | Uniaxial compressive strength of the intact rock (MPa). |
| `gsi` | number 0..100 | yes | Geological Strength Index of the rock mass. |
| `mi` | number ≥ 0.1 | yes | Hoek-Brown constant of the intact rock. |
| `d` | number 0..1 | yes | Disturbance factor (blast damage, stress relaxation). |
| `ei` | number ≥ 0 | yes | Intact rock modulus (MPa) for the generalised Hoek-Diederichs modulus; 0 uses the simplified equation. |
| `application` | one of `tunnel`, `slope` | yes | Range of the Mohr-Coulomb fit: tunnels or slopes (Hoek et al. 2002). |
| `gamma` | number ≥ 1 | yes | Unit weight of the rock mass (kN/m3). |
| `depth` | number ≥ 1 | yes | Depth of the tunnel or height of the slope (m). |

Result `data`: `mb`, `s`, `a`, `sigma_c`, `sigma_t`, `sigma_cm`, `erm` (MPa), `sigma3_max`, `c` (MPa), `phi` (degrees).

### `gt_hb_data` — `MIN_GT_HB_DATA`

Set the Hoek-Brown GSI to the median of the classified records (of the lithology, if given) and sigci / mi / Ei from its laboratory row, then compute like gt_hb. Needs gt_classify first.

| Argument | Type | Setting | Description |
|---|---|---|---|
| `lithology` | text | yes | Lithology whose classified records give the median GSI and whose laboratory row gives σci, mi and Ei. Empty = all records, no laboratory. |

Result `data`: `gsi`, `records` (used for the median), `lab` (true when the lithology had a laboratory row), `sigci`, `mi`, `ei`, and the gt_hb fields under `hoek_brown`.

### `gt_estimate` — `MIN_GT_ESTIMATE`

Interpolate RMR, GSI and log10 Q of the classified records into the block model (the resource model, its .ombm file, or a new one with the resource geometry and one Global domain); writes gt_rmr, gt_gsi, gt_logq, gt_class and the confidence columns gt_dist (mean distance to the records used, m) and gt_n (number of records used). Density and tonnage are not touched. Needs gt_classify first.

| Argument | Type | Setting | Description |
|---|---|---|---|
| `method` | one of `idw`, `nn` | yes | Interpolation method. |
| `power` | number ≥ 0.1 | yes | Inverse distance power. |
| `radius1` | number ≥ 0.1 | yes | Search radius along the azimuth (m). |
| `radius2` | number ≥ 0.1 | yes | Horizontal search radius across the azimuth (m). |
| `radius3` | number ≥ 0.1 | yes | Vertical search radius (m). |
| `azimuth` | number | yes | Azimuth of radius 1 (degrees from north). |
| `min_samples` | number ≥ 1 | yes | Minimum records to estimate a block. |
| `max_samples` | number ≥ 1 | yes | Maximum records used per block. |
| `max_per_hole` | number ≥ 0 | yes | Maximum records per drillhole (0 = no limit). |
| `honour_domains` | boolean | yes | Each record only informs the resource domain whose values list its lithology. |

Result `data`: `estimated`, `blocks` (in a domain), `no_domain` (records skipped by honour_domains), `columns`, `model_created`, `saved` (file path or null).

### `gt_view` — `MIN_GT_VIEW`

Draw the block model coloured by a geotechnical column as level plans, vertical sections (2 points or along an axis) or cubes (mode=cubes: one cube per block with a value, grouped by colour, at most `limit`). Replaces the previous model view.

| Argument | Type | Setting | Description |
|---|---|---|---|
| `mode` | one of `levels`, `two_points`, `axis`, `cubes` | yes | levels = horizontal plans; two_points / axis = vertical sections; cubes = every block with a value as a cube (coloured meshes, one per colour). |
| `attribute` | one of `rmr`, `gsi`, `logq`, `class`, `dist`, `n` | yes | Geotechnical column to colour the block model view with (class uses the class colours; others show poor rock in red). |
| `z_from` | number | yes | Lowest level (mode levels; from = to = every block level). |
| `z_to` | number | yes | Highest level (mode levels). |
| `z_step` | number ≥ 0.1 | yes | Level spacing (mode levels, m). |
| `spacing` | number ≥ 0.1 | yes | Spacing of parallel sections (mode two_points, m). |
| `count` | number ≥ 1 | yes | Number of parallel sections (mode two_points). |
| `interval` | number ≥ 0.1 | yes | Section spacing along the axis (mode axis, m). |
| `limit` | number ≥ 1 | yes | Maximum polygons or cubes drawn; above it cubes are an even sample of the blocks. |
| `show` | one of `all`, `below`, `above` | yes | Show every block, or only those whose attribute value is below / above `threshold` (e.g. attribute=class show=above threshold=3 shows classes IV and V; attribute=rmr show=below threshold=40 shows poor rock). |
| `threshold` | number | yes | Threshold of `show`, in the units of the attribute (RMR, GSI, log Q, class number, metres, records). |
| `cut` | boolean | yes | Keep only the blocks on one side of a plane, to see inside the model (a vertical plane to see a section, a horizontal one to remove the top). |
| `cut_e` | number | yes | East of a point of the cutting plane (world). |
| `cut_n` | number | yes | North of a point of the cutting plane. |
| `cut_z` | number | yes | Elevation of a point of the cutting plane. |
| `cut_dip` | number 0..90 | yes | Dip of the cutting plane (0 = horizontal, 90 = vertical). |
| `cut_dipdir` | number 0..360 | yes | Dip direction of the cutting plane (azimuth); for a vertical plane it is the direction its normal points to. |
| `cut_side` | one of `front`, `back` | yes | front = the side the plane faces (towards its dip direction, or up for a horizontal plane); back = the other side. |
| `axis` | entity handle (hex) |  | Axis polyline (mode axis). Without it a pick is started. |
| `p1` | point `x,y[,z]` |  | First point of the section line (mode two_points). |
| `p2` | point `x,y[,z]` |  | Second point of the section line. |

Positional form: `MIN_GT_VIEW attribute` (`-` skips one).

Result `data`: `attribute`, `column`, `mode`, and the view result: `planes`, `polygons`, `limited`, `cubes` (`[drawn, total]` in cubes mode).

### `st_stereonet` — `MIN_ST_STEREONET`

Equal-area lower-hemisphere stereonet of the discontinuity poles (table and/or traces with dip), density contours and the set planes; chart in the panel.

| Argument | Type | Setting | Description |
|---|---|---|---|
| `joints` | boolean | yes | Use the oriented discontinuities loaded with gt_load. |
| `traces` | boolean | yes | Also use the geological traces (gm_trace) that store a dip and dip direction. |
| `contours` | boolean | yes | Draw pole density contours (1 % counting cone). |

Result `data`: `poles`, `from_table`, `from_traces`, `max_density` (% per 1 %), `sets` (shown).

### `st_sets` — `MIN_ST_SETS`

Group the poles into discontinuity sets (spherical clustering; Fisher K and 95 % cone) and write them into the sets table (friction and cohesion of existing rows are kept).

| Argument | Type | Setting | Description |
|---|---|---|---|
| `n_sets` | integer 0..6 | yes | Sets to find; 0 picks the density peaks that hold at least 3 poles (at most 6). |
| `min_angle` | number 5..90 | yes | Minimum angle between set mean poles. |
| `max_angle` | number 5..90 | yes | Poles farther than this from every set stay unassigned. |
| `sets` | table `Set | Dip | Dip direction | φ (°) | c (kPa)` | yes | Discontinuity sets: rows of [name, dip, dip direction, friction degrees, cohesion kPa]. Filled by st_sets, editable. |

Result `data`: `sets`: list of `{name, dip, dipdir, count, k, alpha95}`, `unassigned`.

### `st_kinematic` — `MIN_ST_KINEMATIC`

Markland kinematic analysis for a slope or excavation face: planar sliding, wedge sliding and flexural toppling (direct toppling not included), for every pole, pole pair and set.

| Argument | Type | Setting | Description |
|---|---|---|---|
| `face_dip` | number 0..90 | yes | Dip of the slope or excavation face (90 = vertical wall). |
| `face_dipdir` | number 0..360 | yes | Dip direction of the face (direction it faces). |
| `friction` | number 0..90 | yes | Friction angle of the discontinuities. |
| `lateral` | number 0..90 | yes | Lateral limits for planar sliding and toppling (usually 20). |

Result `data`: `planar`, `wedge`, `flexural` (`{critical, of, pct}`), `planar_sets`, `wedge_sets` (pairs of set names), `flexural_sets`.

### `st_wedges` — `MIN_ST_WEDGES`

Largest wedges of three sets in the roof and walls of a working (planar faces), with volume, weight, failure mode and factor of safety (Hoek-Bray vector method, support pressure optional); drawn when an axis or point is given.

| Argument | Type | Setting | Description |
|---|---|---|---|
| `axis_trend` | number 0..360 | yes | Trend of the working's axis (ignored when an axis polyline is given). |
| `axis_plunge` | number -90..90 | yes | Plunge of the axis (positive down). |
| `width` | number ≥ 0.1 | yes | Width of the opening (span of roof wedges). |
| `height` | number ≥ 0.1 | yes | Height of the opening (span of wall wedges). |
| `max_length` | number ≥ 0 | yes | Limit of the wedge trace along the axis (e.g. joint trace length). |
| `set_a` | integer 1..20 | yes | First set of the wedge (1-based row of the sets table). |
| `set_b` | integer 1..20 | yes | Second set (row). |
| `set_c` | integer 1..20 | yes | Third set (row). |
| `gamma` | number ≥ 1 | yes | Unit weight of the rock (wedges, in situ stress). |
| `support` | number ≥ 0 | yes | Support pressure on the wedge base (bolts, shotcrete); 0 = none. |
| `chainage` | number ≥ 0 | yes | Horizontal chainage on the axis polyline where the wedges are drawn. |
| `axis` | entity handle (hex) |  | Axis polyline: its direction at `chainage` replaces the trend/plunge and the wedges are drawn there. |
| `at` | point `x,y[,z]` |  | Draw the wedges with the axis passing through this point (floor level per the workings settings). Without axis or at: computed only. |

Result `data`: `sets` (names used), `wedges`: list of `{face, mode, fs, volume, weight, apex_height, base_area}` (fs null = stable); `drawn`.

### `st_draw` — `MIN_ST_DRAW`

Draw the stereonet (net, poles, contours, set great circles) in the DWG as 2D entities.

| Argument | Type | Setting | Description |
|---|---|---|---|
| `radius` | number ≥ 0.1 | yes | Radius of the stereonet drawn in the DWG. |
| `at` | point `x,y[,z]` |  | Centre of the stereonet (world x,y[,z]; default 0,0,0). |

Result `data`: `entities`.

### `sm_stress` — `MIN_SM_STRESS`

In situ stresses from the depth gradient (sigma_v = gamma H, sigma_H, sigma_h by K ratios) and their components in the section of a working with the given trend.

| Argument | Type | Setting | Description |
|---|---|---|---|
| `depth` | number ≥ 1 | yes | Depth below surface of the working or pillars. |
| `gamma` | number ≥ 1 | yes | Unit weight of the overburden. |
| `k_h` | number ≥ 0 | yes | Ratio of the major horizontal to the vertical stress. |
| `k_hmin` | number ≥ 0 | yes | Ratio of the minor horizontal to the vertical stress. |
| `az_h` | number 0..360 | yes | Azimuth of the major horizontal stress. |
| `axis_trend` | number 0..360 | yes | Trend of the working whose section stresses are computed. |

Result `data`: `sigma_v`, `sigma_h_max`, `sigma_h_min`, `section`: `{sigma_x, sigma_z, tau_xz, sigma_1, sigma_3, angle, sigma_axial}` (MPa).

### `sm_kirsch` — `MIN_SM_KIRSCH`

Elastic stresses around a circular opening (Kirsch) in the section stress field: boundary tangential stress, strength factor with the Hoek-Brown rock mass and depth of the overstressed zone; chart in the panel. Non-circular openings use an equivalent radius (approximation).

| Argument | Type | Setting | Description |
|---|---|---|---|
| `opening` | one of `circle`, `equivalent` | yes | Circular opening of `radius`, or the circle with the area of width x height (approximation). |
| `radius` | number ≥ 0.1 | yes | Radius of the circular opening. |

Result `data`: `radius`, `equivalent`, `p1`, `p3`, `sigma_t_max`, `sigma_t_min`, `sigma_cm`, `strength_factor`, `overstressed_depth` (m).

### `sm_convergence` — `MIN_SM_CONVERGENCE`

Convergence-confinement of the working: Duncan Fama ground curve (c', phi' of the Hoek-Brown fit), Vlachopoulos-Diederichs displacement profile, shotcrete and bolt support curves, equilibrium, support factor of safety and Hoek-Marinos squeezing class; chart in the panel.

| Argument | Type | Setting | Description |
|---|---|---|---|
| `e_mass` | number ≥ 0 | yes | Rock mass modulus; 0 takes it from the Hoek-Brown panel. |
| `nu` | number 0..0.49 | yes | Poisson's ratio of the rock mass. |
| `install` | number ≥ 0 | yes | Distance behind the face where the support is installed. |
| `shotcrete` | boolean | yes | Include a shotcrete lining. |
| `sc_thickness` | number ≥ 0.01 | yes | Shotcrete thickness. |
| `sc_strength` | number ≥ 1 | yes | Shotcrete uniaxial compressive strength. |
| `sc_modulus` | number ≥ 100 | yes | Shotcrete Young's modulus. |
| `bolts` | boolean | yes | Include mechanically anchored rock bolts. |
| `b_length` | number ≥ 0.1 | yes | Bolt length. |
| `b_diameter` | number ≥ 0.001 | yes | Bolt diameter. |
| `b_spacing_c` | number ≥ 0.1 | yes | Bolt spacing around the tunnel. |
| `b_spacing_l` | number ≥ 0.1 | yes | Bolt spacing along the tunnel. |
| `b_capacity` | number ≥ 0.001 | yes | Ultimate load of one bolt (MN); use the bolt's value. |
| `b_q` | number ≥ 0 | yes | Deformation-load constant of the anchor and plate (m/MN), from pull tests. |

Result `data`: `radius`, `p0`, `c`, `phi`, `e_mass`, `p_cr`, `plastic_radius`, `u_max` (m), `strain` (%), `squeezing`, `u_install`, `p_eq`, `u_eq`, `support_fs`, `support_p_max`.

### `sm_pillar` — `MIN_SM_PILLAR`

Pillar stress by tributary area (vertical stress of sm_stress), extraction ratio, strength by the chosen empirical formula (all formulas reported) and factor of safety.

| Argument | Type | Setting | Description |
|---|---|---|---|
| `shape` | one of `square`, `rectangular`, `rib` | yes | Pillar shape for the tributary area. |
| `width` | number ≥ 0.1 | yes | Pillar width. |
| `length` | number ≥ 0.1 | yes | Pillar length (rectangular). |
| `height` | number ≥ 0.1 | yes | Pillar height. |
| `room` | number ≥ 0.1 | yes | Width of the rooms between pillars. |
| `ucs` | number ≥ 0 | yes | Intact rock UCS for the strength formula; 0 takes σci of the Hoek-Brown panel. |
| `formula` | one of `lunder_pakalnis`, `obert_duvall`, `bieniawski`, `hedley_grant` | yes | Empirical pillar strength formula. |

Result `data`: `load`, `extraction`, `width_height`, `strength`, `fs`, `formula`, `all` (formula -> `{strength, fs}`).

### `sm_span` — `MIN_SM_SPAN`

Scaled span of a crown pillar (Carter 1992), compared with the critical scaled span you give.

| Argument | Type | Setting | Description |
|---|---|---|---|
| `span` | number ≥ 0.1 | yes | Span of the crown pillar. |
| `strike_length` | number ≥ 0.1 | yes | Length along strike. |
| `thickness` | number ≥ 0.1 | yes | Thickness of the crown pillar. |
| `gravity` | number ≥ 0.1 | yes | Specific gravity of the crown pillar rock. |
| `dip` | number 0..90 | yes | Dip of the orebody / stope walls. |
| `critical` | number ≥ 0 | yes | Critical scaled span from your reference (e.g. Carter's Sc-Q relation); compared with Cs. Not computed by the plugin. |

Result `data`: `scaled_span`, `critical` (null if not given), `ratio` (Sc / Cs).

### `sm_draw` — `MIN_SM_DRAW`

Draw the last Kirsch or convergence-confinement chart in the DWG (frame, axes, curves).

| Argument | Type | Setting | Description |
|---|---|---|---|
| `chart` | one of `kirsch`, `convergence` | yes | Which computed chart to draw. |
| `size` | number ≥ 1 | yes | Width of the drawn chart. |
| `at` | point `x,y[,z]` |  | Lower-left corner of the chart (world; default 0,0,0). |

Result `data`: `entities`.

## Narrow veins

### `vn_surfaces` — `MIN_VN_SURFACES`

Build the hanging-wall and footwall surfaces of a narrow vein from the mapped contact polylines (by layer, or picked as hw= / fw=) and from the vein intercepts of the loaded drillholes and channels; they are interpolated in the vein frame (strike, dip, normal), drawn and saved, and diagnosed: fit to each contact (worst first), nodes where the footwall crosses above the hanging wall, nodes with one wall only, thickness range. Read the diagnostics and adjust step, radius, method or the contacts, then build again.

| Argument | Type | Setting | Description |
|---|---|---|---|
| `vein` | text | yes | Name of the vein (its surfaces and model are stored under it). |
| `hw_layer` | text | yes | Layer of the mapped hanging-wall contact polylines (wildcards * ? allowed). Empty = none. |
| `fw_layer` | text | yes | Layer of the mapped footwall contact polylines (wildcards allowed). Empty = none. |
| `intercepts` | boolean | yes | Also use the vein intervals of the loaded drillholes and channels as hanging-wall / footwall contacts. |
| `vein_attr` | text | yes | Geology attribute that marks the vein intervals (e.g. tipo, lito). |
| `vein_values` | text | yes | Values of that attribute that are vein, separated by ; (e.g. VETA). |
| `orientation` | one of `fit`, `typed` | yes | Plane of the vein frame: fitted to all contacts (least squares) or the dip / dip direction typed. |
| `dip` | number 0..90 | yes | Dip of the reference plane (orientation=typed). |
| `dipdir` | number 0..360 | yes | Dip direction of the reference plane (orientation=typed). |
| `step` | number ≥ 0.1 | yes | Grid spacing on the vein plane. |
| `margin` | number ≥ 0 | yes | Margin of the grid around the contacts. |
| `radius` | number ≥ 0.1 | yes | Search radius; grid nodes farther from the contacts stay empty (maximum extrapolation). |
| `min_samples` | integer 1..50 | yes | Minimum contact points to estimate a node. |
| `max_samples` | integer 1..200 | yes | Maximum contact points used per node. |
| `method` | one of `kriging`, `idw` | yes | Interpolation of the walls. |
| `power` | number ≥ 0.1 | yes | Inverse distance power (method=idw). |
| `range` | number ≥ 0 | yes | Range of the spherical model (method=kriging); 0 uses the search radius. |
| `densify` | number ≥ 0 | yes | Points taken along the mapped polylines at least every this distance (0 = vertices only). |
| `min_thickness` | number ≥ 0 | yes | Nodes thinner than this are reported as thin (diagnostic only). |
| `hw` | handles `1A,2B` |  | Hanging-wall contact polylines picked instead of (or in addition to) the layer. |
| `fw` | handles `1A,2B` |  | Footwall contact polylines picked instead of (or in addition to) the layer. |

Positional form: `MIN_VN_SURFACES vein` (`-` skips one).

Result `data`: `vein`, `plane` (`{dip, dipdir, origin}`), `contacts` (`{hanging_wall, footwall, from_layers, from_handles, from_intercepts}`), `grid` (`{nx, ny, step, nodes}`), `diagnostics` (`{both, one_side, crossings, thin, thickness_min, thickness_mean, thickness_max, rms_hw, rms_fw, unchecked}`), `worst` (list of `{source, side, residual, point}`), `file` (saved surfaces or null).

### `vn_variogram` — `MIN_VN_VARIOGRAM`

Composites the vein intercepts of the loaded drillholes and channels (grade weighted by length, true thickness from the thickness column), computes the variograms of the accumulation (grade × thickness) on the vein plane (omnidirectional, along strike, down dip), fits a model and stores nugget, sill and both ranges for vn_estimate.

| Argument | Type | Setting | Description |
|---|---|---|---|
| `vein` | text | yes | Name of the vein (its surfaces must be built first). |
| `grade` | text | yes | Assay column estimated (e.g. au, ag, cu%). |
| `thickness_col` | text | yes | Assay column with the true vein thickness (channels: potencia). Empty or missing = length of the intercept along the vein normal. |
| `lag` | number ≥ 0.1 | yes | Lag of the experimental variograms on the vein plane. |
| `nlags` | integer 2..50 | yes | Number of lags. |
| `kind` | one of `spherical`, `exponential`, `gaussian` | yes | Variogram model type. |

Positional form: `MIN_VN_VARIOGRAM vein` (`-` skips one).

Result `data`: `vein`, `composites`, `missing_grade`, `stats` (`{grade_mean, thickness_mean, acc_mean, acc_variance}`), `experimental` (`{omni, strike, dip}`: lists of `{lag, gamma, pairs}`), `model` (`{kind, nugget, sill, range_strike, range_dip}`).

### `vn_estimate` — `MIN_VN_ESTIMATE`

Builds the prism model of the vein: a grid of prisms on the vein plane between the hanging-wall and footwall surfaces; accumulation and thickness are block-kriged with the same weights and grade = accumulation / thickness. The boundary polygon, the maximum distance and the long-section edits limit or override the prisms. Draws the prisms, saves them as CSV, redraws the long section if drawn, and validates against the composites (cross-validation and composite vs prism grade).

| Argument | Type | Setting | Description |
|---|---|---|---|
| `vein` | text | yes | Name of the vein (its surfaces must be built first). |
| `grade` | text | yes | Assay column estimated (e.g. au, ag, cu%). |
| `thickness_col` | text | yes | Assay column with the true vein thickness (channels: potencia). Empty or missing = length of the intercept along the vein normal. |
| `kind` | one of `spherical`, `exponential`, `gaussian` | yes | Variogram model type. |
| `nugget` | number ≥ 0 | yes | Nugget of the accumulation variogram (vn_variogram fills it). |
| `sill` | number ≥ 0 | yes | Partial sill of the accumulation variogram. |
| `range_strike` | number ≥ 0.01 | yes | Range along strike. |
| `range_dip` | number ≥ 0.01 | yes | Range down dip. |
| `block_u` | number ≥ 0.1 | yes | Prism size along strike. |
| `block_v` | number ≥ 0.1 | yes | Prism size down dip. |
| `search_radius` | number ≥ 0.1 | yes | Search radius on the vein plane. |
| `min_samples` | integer 1..50 | yes | Minimum composites to estimate a prism. |
| `max_samples` | integer 1..200 | yes | Maximum composites per prism. |
| `max_distance` | number ≥ 0 | yes | Prisms farther than this from every composite are left out. |
| `disc` | integer 1..10 | yes | Points per side used for block kriging. |
| `density` | number ≥ 0.1 | yes | Density of the vein. |
| `color_by` | one of `grade`, `thickness`, `accumulation`, `distance`, `variance` | yes | Attribute that colours the prisms and the long section. |
| `draw_prisms` | boolean | yes | Draw the prisms between the walls. |

Positional form: `MIN_VN_ESTIMATE vein` (`-` skips one).

Result `data`: `vein`, `summary` (`{prisms, area, volume, tonnes, grade, thickness, edited, out_boundary, out_distance, out_walls, out_edits, not_estimated}`), `validation` (`{cv_n, cv_mean_error, cv_correlation, n, mean_composite, mean_prism, correlation, worst: [{hole, composite, prism}]}`), `composites`, `file`.

### `vn_long` — `MIN_VN_LONG`

Draws the long section of the vein model flat in the drawing (along strike × up dip): prisms coloured by the chosen attribute, composites with labels, the boundary and the edit polygons. Polygons drawn on it are used by vn_edit.

| Argument | Type | Setting | Description |
|---|---|---|---|
| `vein` | text | yes | Name of the vein (its surfaces must be built first). |
| `color_by` | one of `grade`, `thickness`, `accumulation`, `distance`, `variance` | yes | Attribute that colours the long section. |
| `labels` | boolean | yes | Write hole and grade beside each composite. |
| `at` | point `x,y[,z]` |  | Lower-left corner of the long section in the drawing. Default: the last one, or beside the model. |

Positional form: `MIN_VN_LONG vein` (`-` skips one).

Result `data`: `vein`, `at` (base point), `u0`, `v0`, `width`, `height`, `prisms`, `composites`.

### `vn_edit` — `MIN_VN_EDIT`

Edits the prism model with a polygon drawn on the long section (polygon=handle, else the selection, else it asks to pick it): exclude prisms, set their grade or thickness, or set the boundary polygon; undo the last edit, clear all, or list them. The model is estimated again after each change.

| Argument | Type | Setting | Description |
|---|---|---|---|
| `vein` | text | yes | Name of the vein (its surfaces must be built first). |
| `action` | one of `exclude`, `grade`, `thickness`, `boundary`, `undo`, `clear`, `list` | yes | What to do with the polygon drawn on the long section. |
| `polygon` | entity handle (hex) |  | Closed polyline drawn on the long section (exclude, grade, thickness, boundary). |
| `value` | number | yes | Grade or thickness for action=grade / thickness. |

Positional form: `MIN_VN_EDIT action` (`-` skips one).

Result `data`: `vein`, `edits` (list of `{action, value, vertices}`), `boundary` (vertex count), and the vn_estimate result in `estimate` (null for list).

