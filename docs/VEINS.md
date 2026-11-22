# Narrow veins

[Español](es/VETAS.md) · [Example with numbers](../examples/veta/README.md) · [Operations for AI](MCP.md)

Narrow veins are not modelled with the regular block model. The vein model is
built from data in stages, each an operation an AI assistant can run, check and
adjust (see [MCP.md](MCP.md)):

| Stage | Status | What it does |
|---|---|---|
| A — channels | done | Channel samples of the workings converted into drillhole data (`MIN_CH_CONVERT`, see [examples/canales](../examples/canales/README.md)) |
| B1 — surfaces | done | Hanging-wall and footwall surfaces from mapped contacts and vein intercepts (`MIN_VN_SURFACES`) |
| B2 — prisms and grades | done | 3D prisms between the surfaces, grades by accumulation on the vein plane, in-plane variography, validation against channels, boundary polygon, maximum distance, edits on the long section (`MIN_VN_VARIOGRAM`, `MIN_VN_ESTIMATE`, `MIN_VN_LONG`, `MIN_VN_EDIT`) |
| B3 — tonnage | done | Dilution and recovery, blocks between levels and raises with pillars, sampled sides, categories by an ordered rule table managed from MCP (`MIN_VN_CUBICATE`, `MIN_VN_RULES`, `MIN_VN_BUILD`) |
| B4 — branches | planned | Several veins, overlaps reported, incremental update when channels are added |

**Order of work:** channels (or drillholes) loaded → surfaces → variogram → prisms → (long section, edits) →
tonnage. `MIN_VN_BUILD` runs everything after the data load in one call; each stage can also be re-run alone
after changing a setting. Every stage reads the stored settings of the vein (panel values), so what you set in a
panel, in a command or through an assistant is the same.

**Layers and files**

| Layer | Content | Colour |
|---|---|---|
| `VETA_TECHO`, `VETA_PISO` | *your* mapped contact polylines (the names are settings) | — |
| `NIVELES`, `CHIMENEAS` | *your* level and raise polylines (settings) | — |
| `MIN_VN_TECHO` / `MIN_VN_PISO` | hanging-wall / footwall surfaces | 30 / 4 |
| `MIN_VN_PRISMAS` | prisms | by attribute |
| `MIN_VN_LONG` | long section, samples, edit polygons | 7 |
| `MIN_VN_BLOQUES` | workings and block labels on the long section | 1 |

Files next to the drawing: `<drawing>_veta_<name>.json` (surfaces), `_prismas.csv`, `_bloques.csv`.

## B1 — Hanging-wall and footwall surfaces

Panel **Veta: superficies** (ribbon group *Vetas*), command `MIN_VN_SURFACES`.

**Contacts**

- Mapped contacts: polylines (3D polylines, polylines, lines) on the hanging-wall
  and footwall layers (default `VETA_TECHO` / `VETA_PISO`, wildcards allowed) and/or
  picked as `hw=` / `fw=` handles. Points are taken at the vertices and at least every
  *densify* metres along them. They are drawing entities: the local origin of the
  drawing is added back to work in world coordinates.
- Vein intercepts: the geology intervals of the loaded drillholes and channels whose
  attribute (`vein_attr`, default `tipo`) has one of the vein values (`vein_values`,
  default `VETA`), merged per hole. Each intercept gives one hanging-wall and one
  footwall point (the end farther along the vein normal is the hanging wall).

**Vein frame.** A reference plane fitted by least squares to every contact (or typed
dip / dip direction) defines local coordinates: `u` along strike (horizontal), `v` up
dip and `w` along the normal towards the hanging wall. Steep and vertical veins are
handled like flat ones.

**Surfaces.** The normal offset `w` of each wall is interpolated on a common grid of
`step` metres over the contacts plus a margin, by ordinary kriging (spherical model,
sill = variance of the offsets, range = the search radius unless given) or inverse
distance. Nodes with fewer than `min_contacts` contacts within the search radius stay
empty: the radius is the maximum extrapolation. Both walls are drawn as meshes on
`MIN_VN_TECHO` and `MIN_VN_PISO` (XDATA `VN_SURFACE`: `vein`, `side`) and saved as
`<drawing>_veta_<name>.json` for the prism model.

**Diagnostics** (in the command line, the panel and the result JSON):

- fit to the contacts: RMS per wall and the ten contacts farthest from their surface
  (source polyline or hole, residual in metres);
- crossings: nodes where the footwall is above the hanging wall (bad contact,
  wrong side or a pinch-out to bound with a polygon in B2);
- nodes with one wall only (widen the radius or add contacts);
- thickness range between the walls, and thin nodes below *min_thickness*.

An assistant uses them to iterate: e.g. a large residual on one contact → check or
drop that polyline; crossings near a hole → its vein interval may be mislogged;
many one-sided nodes → larger radius or more mapping.

## B2 — Prism model and grades

Panel **Veta: modelo de prismas** (ribbon group *Vetas* → *Modelo*).

**Composites.** Each vein intercept of the loaded drillholes and channels (B1) is one
sample on the vein plane: its grade (`grade`, e.g. `au`) is the length-weighted mean of
the assays inside the intercept; its true thickness is the length-weighted mean of the
thickness column (`thickness_col`, channels: `potencia`), or the intercept length along
the vein normal when that column is missing; accumulation = grade × thickness.

**In-plane variography** (`MIN_VN_VARIOGRAM`). Variograms of the accumulation on the
vein plane: omnidirectional, along strike and down dip (±22.5°), with a fitted model
(spherical, exponential or Gaussian) whose nugget, sill and ranges along strike and
down dip are written to the settings (edit them before estimating if needed). The chart
is shown in the panel.

**Estimation** (`MIN_VN_ESTIMATE`). A grid of prisms of `block_u` × `block_v` metres on
the vein plane covers the surfaces. For each prism, accumulation and thickness are
block-kriged (ordinary kriging, `disc` × `disc` discretisation) with the same samples
and weights, and grade = accumulation / thickness. The prism volume comes from the two
walls at its corners (B1); tonnes = volume × density. Prisms are left out when they are
outside the boundary polygon, farther than `max_distance` from every composite, where a
wall is missing or the footwall is above the hanging wall, inside an *exclude* edit, or
without `min_samples` composites in the search radius — each count is reported.

Results: prisms drawn as closed meshes on `MIN_VN_PRISMAS` (XDATA `VN_PRISM`: `vein`,
`i`, `j`, grade, `thickness`, `tonnes`) coloured by grade, thickness, accumulation,
distance to the nearest sample or kriging variance; a CSV
`<drawing>_veta_<name>_prismas.csv` (`i, j, u, v, x, y, z, thickness, area, volume,
tonnes, <grade>, accumulation, distance, samples, variance, edited`); and the totals
(area, volume, tonnes, mean grade and thickness).

**Validation against the samples**: cross-validation of the accumulation hiding each
sample's own hole (bias and correlation), and each composite against the prism that
contains it (means, correlation and the largest differences, by hole).

**Long section and edits** (`MIN_VN_LONG`, `MIN_VN_EDIT`). The long section is drawn flat
in the drawing (along strike to the right, up dip upwards) at a base point (`at=`, by
default beside the model, then the last one): prisms as coloured faces, composites with
`hole grade/thickness` labels, the boundary (yellow) and the edit polygons (magenta), on
`MIN_VN_LONG`. Draw a closed polyline on it and run `MIN_VN_EDIT` with an action:

| action | effect |
|---|---|
| `exclude` | prisms inside are left out (waste, mined out, pillar) |
| `grade` | prisms inside take `value` as grade |
| `thickness` | prisms inside take `value` as thickness (centred on the vein) |
| `boundary` | the polygon becomes the limit of the model |
| `undo` / `clear` / `list` | remove the last edit / all edits and the boundary / list them |

Edits are kept per vein in the project settings, applied in order on every estimate, and
the model and long section are updated after each edit.

## B3 — Tonnage, blocks and categories

Panel **Veta: cubicación** (ribbon group *Vetas* → *Cubicación*), command `MIN_VN_CUBICATE`,
after `MIN_VN_ESTIMATE`.

**Dilution and recovery** (per prism). A prism thinner than the minimum mining width
(`mmw`) is widened to it, half on each wall; the hanging-wall and footwall overbreak
(`overbreak_hw`, `overbreak_fw`) are added on top. The added rock has its own density
(`density_waste`) and grade on each wall (`grade_hw`, `grade_fw`):

```
diluted tonnes = area × (thickness × density + (hw + fw) × waste density)
diluted grade  = (ore t × grade + hw t × grade_hw + fw t × grade_fw) / diluted tonnes
recovered      = diluted tonnes × recovery
```

**Blocks between workings.** Levels (drifts) and raises are polylines on their layers
(default `NIVELES` / `CHIMENEAS`) or picked (`levels=`, `raises=`); they are projected on
the vein plane. Levels are numbered bottom-up `L1, L2, …` and raises left to right
`R1, R2, …` along strike; each prism belongs to the panel between them, named
`L1-L2/R1-R2` (`<L1`, `>L2`, `<R1`, `>R1` outside the first or last working). Prisms
inside the sill pillar above each level (`level_pillar` m) or the pillar centred on each
raise (`raise_pillar` m wide) are kept apart and reported as pillars.

**Sampled sides.** Each side of a block (lower level, upper level, left raise, right
raise) is sampled when at least `side_min` composites lie within `side_tolerance` metres
of that working inside the block's span. A side without a working (an open edge of the
model) does not count. The result lists the sides as `true` / `false` / `null`.

**Categories by rules** (managed from MCP). The table `rules` is an ordered list of
`[category, min sampled sides, max mean distance, min samples, min diluted grade]`; an
empty cell is not checked. Each block takes the **first** rule it meets; blocks that meet
none are unclassified. The plugin ships example rules (Medido 3 sides ≤ 15 m, Indicado
2 sides ≤ 30 m, Inferido 1 side) — replace them with the company's norms, from the panel
or with `rules=[[...], ...]`. `MIN_VN_RULES sides=2 dist=20 samples=4 grade=3.5` tells
which rule a block with those values meets and why every earlier rule fails, so norms can
be tested before cubicating. Each block of the result also carries `why`.

**Outputs.** Command line and panel: one line per block (category, in-situ, diluted and
recovered tonnes and grades, sampled sides ■ □ ·, mean distance) and totals per category;
the result JSON (`blocks`, `totals`, `pillars`); `<drawing>_veta_<name>_bloques.csv`; on the
long section, the workings (red) and the block labels coloured by category
(`MIN_VN_BLOQUES`, XDATA `VN_BLOCK`). With *Colorear por* = *Categoría* the long section
is coloured by category too.

**Build all** (`MIN_VN_BUILD`): runs `vn_surfaces`, `vn_variogram` (if `fit_variogram`),
`vn_estimate` and `vn_cubicate` (if `tonnage`), stops at the first failing step and names it. It
accepts every argument of those steps (`step=`, `radius=`, `grade=`, `block_u=`, `max_distance=`,
`mmw=`, `levels=`, `rules=`…); the ones you give are stored, the others keep their current value.

## Reading the diagnostics

| You see | It means | What to do |
|---|---|---|
| `crossings > 0` (surfaces) | footwall above the hanging wall at some nodes | look at `worst`: a contact on the wrong wall or a mislogged `tipo` interval; fix or remove it |
| `one_side > 0` | a wall is defined where the other is not | raise `radius`, or map / sample more there |
| large `rms_hw` / `rms_fw`, big `worst[0].residual` | the surface does not follow some contact | check that contact (source name); try the other `method` or a smaller `radius` |
| `thin > 0` | nodes thinner than `min_thickness` | pinch-out: bound it with a polygon, or accept it |
| `from_intercepts = 0` | the vein intervals were not found | check `vein_attr` / `vein_values` against the geology column, and that channels / holes are loaded |
| `composites < 3` | too few intercepts with grade | load more data; `missing_grade` says how many have no assay |
| `cv_correlation` near 0, high nugget | the grades have little spatial continuity at this spacing | treat local grades with caution and look at the variogram (nugget, range) |
| `mean_composite` ≠ `mean_prism` by > 10 % | the estimate is biased against the samples | check the variogram ranges, `max_distance`, clustered samples; `worst` names the holes |
| `out_distance`, `out_walls` large | many cells dropped | raise `max_distance` or `radius`, or accept the limit |
| a block is not in the category you expect | a rule failed | read the block's `why`; test with `MIN_VN_RULES` |

## Limits

- One vein at a time (`vein=` names it; several can be kept in the same drawing, one after another).
  Branches and overlap checks between veins are planned (B4).
- Contacts and workings must exist as polylines (or come from intercepts); the plugin does not trace the
  contacts by itself.
- Grades are interpolated, not simulated; there are no resource classes by kriging variance: categories come from
  your rule table.
- The rules shipped in the table are examples, not a standard.
