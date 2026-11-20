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

## Drillholes

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
| `attribute` | one of `grade_ok`, `ok_variance`, `kriging_efficiency`, `domain`, `pass`, `samples`, `distance`, `nn`, `idw`, `sk`, `tonnage`, `gt_rmr`, `gt_gsi`, `gt_logq`, `gt_class` | yes | Column that colours the blocks. |
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

Interpolate RMR, GSI and log10 Q of the classified records into the block model (the resource model, its .ombm file, or a new one with the resource geometry and one Global domain); writes gt_rmr, gt_gsi, gt_logq, gt_class. Density and tonnage are not touched. Needs gt_classify first.

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

Draw the block model coloured by a geotechnical column, with the view settings of the resource Model panel (levels, sections, cubes).

| Argument | Type | Setting | Description |
|---|---|---|---|
| `attribute` | one of `rmr`, `gsi`, `logq`, `class` | yes | Geotechnical column to colour the block model view with (class uses the class colours; others show poor rock in red). |

Positional form: `MIN_GT_VIEW attribute` (`-` skips one).

Result `data`: `attribute`, `column`; the drawing itself is reported by the resource view.

