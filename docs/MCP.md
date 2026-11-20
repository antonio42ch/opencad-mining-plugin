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

