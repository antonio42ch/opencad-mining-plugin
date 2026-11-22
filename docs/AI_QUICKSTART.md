# Open Mining for AI assistants — quick start

For an assistant (or the person configuring one) that will drive Open Mining through MCP.
Read this first, then use [MCP.md](MCP.md) as the reference (every operation, with a worked example) and
[../AGENTS.md](../AGENTS.md) for the rules. Humans: [GUIDE.md](GUIDE.md).

## 1. Connect

Open CAD Studio exposes the open drawing to MCP clients when started as `OpenCADStudio --mcp` (stdio). In
an MCP client configuration that is, for example:

```json
{ "mcpServers": { "opencad": { "command": "OpenCADStudio", "args": ["--mcp"] } } }
```

(adapt the command to where Open CAD Studio is installed). The plugin must be installed
([INSTALL.md](INSTALL.md)). You then have the host's tools: run a command (`ocs_run`), read the command
**history**, query entities, selection and state. Open Mining adds no tool of its own: every action is a
**command** you run with `ocs_run`, e.g. `MIN_VN_BUILD vein=V1`.

Files you pass (`collar=…`, `sheet=…`) are opened **by the host process**: use paths that exist on the
machine where Open CAD Studio runs.

## 2. Conventions

**Calling.** `MIN_<NAME> key=value key=value …`. Quote values with spaces: `sheet="C:\My data\ch.csv"`.
Booleans `true|false`; points `x,y,z`; entity handles are hex strings (`1A3F`), lists `1A3F,1A40`; tables are
JSON arrays of rows: `rules=[["Medido",3,15,"",""],["Inferido",1,"","",""]]`. The first arguments of some
commands may be given without a key (shown as *Positional form* in MCP.md). `MIN_HELP op=<name>` returns the
exact parameters, types and JSON Schema of any operation.

**Settings.** An argument marked *setting* in MCP.md is a project setting (the value shown in the panel):
if you omit it the **current value** is used, if you give it the value is **stored** for the next run (and
the panel shows it). So a second call only needs what changes. There is no separate "read a setting" call: when
you need to be sure of a value, pass it explicitly, and say in your report which values you set.

**Results.** Each command writes its usual messages and **one result line** to the history:

```
OPENMINING_RESULT {"op":"vn_surfaces","ok":true,"status":"done","data":{…},"created":["1A3F"],"warnings":[]}
OPENMINING_RESULT {"op":"vn_surfaces","ok":false,"error":{"code":"no_data","message":"…","param":"hw"}}
```

Search the history for `OPENMINING_RESULT ` (the host may put an icon before it) and parse the JSON after it.
`status` is `done`, or `waiting_input` when an input you did not pass started an on-screen pick (pass it as
an argument instead). `created` lists the handles of the entities drawn; `warnings` is for the user.
Error `code`s are in [MCP.md](MCP.md#error-codes); `param` names the argument at fault. After `no_data`, call
`MIN_STATUS`.

**Coordinates.** Drillholes, channels and the vein data are in **world** coordinates. If the project has a
*local origin* (`dh_load` returns `origin`), what is *drawn* is shifted by it. Polylines you draw (contacts,
levels, raises) are in drawing coordinates; the plugin converts them. Typed coordinates in `3DPOLY` are
drawing coordinates: subtract the origin when it is enabled.

**Everything is undoable and re-runnable.** One command is one undo step. Running a drawing command again
replaces what the plugin drew before for the same thing; it never touches your own entities.

## 3. Discover

```
MIN_STATUS          → what is loaded and `next`: suggested operations with the reason
MIN_HELP            → list of operations (group=veins limits it)
MIN_HELP op=vn_build  → parameters, schema, result
MIN_HELP workflows=true → typical command sequences and the error codes
MIN_ENTITIES class=VN_PRISM limit=5  → what the plugin drew (decoded XDATA)
```

`MIN_STATUS` reports `veins` (`name`, `channels`, `surfaces`, `prisms`, `tonnage`, `rules`) and, in `next`,
the following step of the vein model when there is one.

## 4. Workflow: drillholes

```
MIN_DH_LOAD collar="…/collar.csv" survey="…/survey.csv" assays="…/assays.csv"
MIN_DH_LIST
MIN_DH_DRAW assay_attr=au
```

Read `errors` and `warnings` of `dh_load` (rules R01–R07) before going on: hole ids that do not match between
tables, depths beyond the collar depth and missing surveys are reported there, not fixed.

## 5. Workflow: narrow vein from channel samples

The complete example with the numbers to expect is [../examples/veta](../examples/veta/README.md).

**a. Channels as drillholes**

```
MIN_CH_CONVERT points="…/puntos_topograficos.csv" sheet="…/muestreo_canales.csv"
```

Check `data.channels`, `samples` and that `issues` is empty (each issue names the row and the cause). The
holes `CH-<channel>` are added to the loaded drillholes. Orient one channel with
`MIN_CH_ORIENT channel=<id> p1=x,y,z p2=x,y,z` when the default (horizontal, perpendicular to the working) is
wrong.

**b. Levels and raises** (only for the tonnage). Draw them as 3D polylines and keep their handles:

```
3DPOLY 500000,8500000,3950 500075,8500129.9,3951.2      (run, then find the new Polyline3D handle)
```

**c. The whole model**

```
MIN_VN_BUILD vein=EJ1 vein_attr=tipo vein_values=VETA grade=au thickness_col=potencia levels=<h1>,<h2> raises=<h3>
```

`vein_attr` / `vein_values` say which geology interval is the vein (`tipo = VETA` in the example sheet);
`grade` is the assay column to estimate; `thickness_col` is the true-thickness column. Contacts mapped
as polylines on layers (`hw_layer=`, `fw_layer=`) or picked (`hw=`, `fw=`) are used in addition to the intercepts.
`MIN_VN_BUILD` accepts the arguments of every step; stages can also be run alone
(`vn_surfaces`, `vn_variogram`, `vn_estimate`, `vn_cubicate`).

**d. Read the diagnostics before reporting anything**

| Step | Fields to read | Healthy | If not |
|---|---|---|---|
| `surfaces` | `plane` (dip, dipdir), `contacts`, `diagnostics.crossings`, `one_side`, `rms_*`, `worst` | plane plausible for the deposit, `from_intercepts` > 0, 0 crossings, small RMS | see the table in [VEINS.md](VEINS.md#reading-the-diagnostics): fix the contact named in `worst`, raise `radius`, check `vein_attr`/`vein_values` |
| `variogram` | `composites`, `model` (nugget, sill, ranges), `stats` | ≥ 3 composites (more is much better), nugget not ≫ sill | a nugget close to the total sill means little spatial continuity: say so |
| `estimate` | `summary` (`prisms`, `tonnes`, `out_*`, `not_estimated`), `validation` (`cv_correlation`, `mean_composite` vs `mean_prism`, `worst`) | prism mean ≈ composite mean; `out_*` explained | change `max_distance`, `radius`, the model or the edits; report the validation as it is |
| `cubicate` | `blocks[].sides`, `category`, `why`, `totals`, `pillars`, `warnings` | every block categorised by a rule you were given | read `why`; test rules with `MIN_VN_RULES` |

**e. Iterate.** Change one setting, run the stage again (or `vn_build`), compare the numbers. Polygons drawn on
the long section (`MIN_VN_LONG`) edit the prisms with `MIN_VN_EDIT action=exclude|grade|thickness|boundary
polygon=<handle>`; `action=list` shows the edits and `clear` removes them.

**f. Categories are the user's norm.** `MIN_VN_RULES` shows the rule table. The rules that ship are **examples**
(Medido 3 sides ≤ 15 m, Indicado 2 sides ≤ 30 m, Inferido 1 side). Ask the user for their norm and set it:

```
MIN_VN_RULES rules=[["Medido",3,15,"",""],["Indicado",2,30,"",""],["Inferido",1,"","",""]]
MIN_VN_RULES sides=2 dist=20          → which rule a block with those values meets, and why the earlier ones fail
MIN_VN_CUBICATE vein=EJ1
```

Columns: category, minimum sampled sides, maximum mean distance (m), minimum samples, minimum diluted grade;
empty = not checked; the first rule a block meets wins.

## 6. Other modules

Resources (`MIN_RS_*`, `MIN_BM_*`) and geotechnics (`MIN_GT_*`, `MIN_ST_*`, `MIN_SM_*`) follow the same
conventions; their sequences are in [MCP.md](MCP.md#typical-workflows), the methods in
[RESOURCES.md](RESOURCES.md) and [GEOTECH.md](GEOTECH.md). The read-only operations (`MIN_STATUS`, `MIN_HELP`,
`MIN_ENTITIES`, `MIN_DH_LIST`, `MIN_RS_STATS`, `MIN_BM_SAMPLE`, `MIN_BM_RESOURCES`…) never change the drawing.

## 7. Before you report a result

1. Say which **inputs** you used (files, layers, handles) and which **settings** differ from the defaults.
2. Quote the **diagnostics** that support the number (RMS, crossings, validation, `out_*`) and any warning.
3. Report limits honestly: few composites, high nugget, blocks far from samples, categories from example rules.
4. Never present an estimate as better validated than `validation` shows; the example data are synthetic.
5. Tell the user what you drew (layers) and that Ctrl+Z undoes each command.
