# Open Mining — cheat sheet

[Español](es/COMANDOS.md) · [User guide](GUIDE.md) · full reference with arguments and results: [MCP.md](MCP.md)

Every command can be typed in the command line as `MIN_X key=value …`, run from its ribbon button, or sent by an
AI assistant. `MIN_HELP` lists them; `MIN_HELP op=<name>` describes one; `MIN_STATUS` says what to do next.

| Tab group | Commands | What they do |
|---|---|---|
| **Project** | `MIN_STATUS` `MIN_HELP` `MIN_ENTITIES` | state and next steps, operation help, entities the plugin drew |
| **Drillholes** | `MIN_DH_PANEL` `MIN_DH_LOAD` `MIN_DH_LIST` `MIN_DH_DRAW` `MIN_DH_CLEAR` `MIN_DH_GEOCOLORS` | load CSV/XLSX tables, validate (R01–R07), list, draw 3D traces by grade or geology, clear, build the geology colour table |
| **Channels** | `MIN_CH_PANEL` `MIN_CH_CONVERT` `MIN_CH_ORIENT` | channel sampling sheets → drillhole data `CH-…`; orient a channel (two clicks or `p1=` `p2=`) |
| **Veins — surfaces** | `MIN_VN_PANEL` `MIN_VN_SURFACES` | hanging-wall and footwall surfaces from contacts and intercepts, with diagnostics |
| **Veins — model** | `MIN_VN_MODEL_PANEL` `MIN_VN_VARIOGRAM` `MIN_VN_ESTIMATE` `MIN_VN_LONG` `MIN_VN_EDIT` | in-plane variogram, prisms with grades, validation, long section, polygon edits |
| **Veins — tonnage** | `MIN_VN_TONNAGE_PANEL` `MIN_VN_CUBICATE` `MIN_VN_RULES` `MIN_VN_BUILD` | dilution and recovery, blocks between levels and raises, categories by your rules; build everything |
| **Workings** | `MIN_UG_PANEL` `MIN_UG_AXIS` `MIN_UG_HBOUND` `MIN_UG_VBOUND` | closed solids from an axis + section, or from four boundary polylines |
| **Axes** | `MIN_AX_PANEL` `MIN_AX_LINE` `MIN_AX_CURVE` `MIN_AX_UNDO` `MIN_AX_NEW` `MIN_AX_PICKSTART` `MIN_AX_FROMPOLY` | working axes as straights and curves with grade |
| **Sections** | `MIN_SC_PANEL` `MIN_SC_RUN` `MIN_TR_LIST` | cut solids — and a terrain (`terrain=`, a Civil TIN surface: profile and contours) — by planes: perpendicular to an axis, two points, levels, three points; list the terrains |
| **Resources** | `MIN_RS_COMP_PANEL` `MIN_RS_VARIO_PANEL` `MIN_RS_BM_PANEL` `MIN_RS_COMPOSITE` `MIN_RS_STATS` `MIN_RS_VARIO` `MIN_RS_FIT` `MIN_RS_MODELSAVE` `MIN_RS_ESTIMATE` `MIN_BM_RESOURCES` `MIN_BM_SAMPLE` `MIN_RS_BMVIEW` | composites, domains, variograms, kriging / IDW, block model, resources and views — see [RESOURCES.md](RESOURCES.md) |
| **Geotechnics** | `MIN_GT_DATA_PANEL` `MIN_GT_HB_PANEL` `MIN_GT_MODEL_PANEL` `MIN_GT_LOAD` `MIN_GT_CLASSIFY` `MIN_GT_HB` `MIN_GT_ESTIMATE` `MIN_GT_VIEW` | RMR89 / Q / GSI, Hoek-Brown, geotechnical block model and views — see [GEOTECH.md](GEOTECH.md) |
| **Structures & design** | `MIN_RM_STRUCT_PANEL` `MIN_RM_DESIGN_PANEL` `MIN_ST_*` `MIN_SM_*` | stereonets, sets, kinematics, wedges; stress, Kirsch, convergence, pillars |
| **Geology** | `MIN_GM_PANEL` `MIN_GM_TRACE` `MIN_GM_TRACE_PICK` `MIN_GM_PROJECT` | traces from coordinates + bearing, projection along the dip |

## Narrow vein in four lines

```
MIN_CH_CONVERT points="…/puntos_topograficos.csv" sheet="…/muestreo_canales.csv"
MIN_VN_BUILD vein=V1 vein_attr=tipo vein_values=VETA grade=au thickness_col=potencia levels=<h1>,<h2> raises=<h3>
MIN_VN_RULES sides=2 dist=20
MIN_VN_CUBICATE vein=V1
```

## Settings worth knowing (veins)

| Argument | Default | Meaning |
|---|---|---|
| `vein_attr`, `vein_values` | `tipo`, `VETA` | geology column and values that mark the vein intervals |
| `grade`, `thickness_col` | `au`, `potencia` | assay column to estimate, true-thickness column |
| `hw_layer`, `fw_layer` | `VETA_TECHO`, `VETA_PISO` | layers of mapped contact polylines |
| `step`, `radius`, `margin` | 2, 50, 5 m | surface grid, search radius (= maximum extrapolation), margin |
| `block_u`, `block_v` | 5, 5 m | prism size along strike and down dip |
| `max_distance` | 40 m | prisms farther from every sample are left out (0 = no limit) |
| `density` | 2.7 t/m³ | vein density |
| `mmw`, `overbreak_hw`, `overbreak_fw`, `recovery` | 0.8 m, 0.10 m, 0.05 m, 90 % | dilution and recovery |
| `level_layer`, `raise_layer` | `NIVELES`, `CHIMENEAS` | layers of levels and raises |
| `rules` | examples | **category rules: replace with your norm** |

## Layers the plugin creates

`MIN_DH_*`, `MIN_RS_COMP_*`, `MIN_RS_*` (drillholes, composites) · `MIN_UG_SOLIDO` (workings) · `MIN_SC_CORTE` (sections) ·
`MIN_GM_TRAZA`, `MIN_GM_PROYECCION` (geology) · `MIN_GT_*` (geotechnics) · `MIN_VN_TECHO`, `MIN_VN_PISO`,
`MIN_VN_PRISMAS`, `MIN_VN_LONG`, `MIN_VN_BLOQUES` (veins). Full list with colours and XDATA classes:
[PLUGIN.md](PLUGIN.md).
