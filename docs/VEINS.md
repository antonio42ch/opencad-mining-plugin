# Narrow veins

Narrow veins are not modelled with the regular block model. The vein model is
built from data in stages, each an operation an AI assistant can run, check and
adjust (see [MCP.md](MCP.md)):

| Stage | Status | What it does |
|---|---|---|
| A — channels | done | Channel samples of the workings converted into drillhole data (`MIN_CH_CONVERT`, see [examples/canales](../examples/canales/README.md)) |
| B1 — surfaces | done | Hanging-wall and footwall surfaces from mapped contacts and vein intercepts (`MIN_VN_SURFACES`) |
| B2 — prisms and grades | done | 3D prisms between the surfaces, grades by accumulation on the vein plane, in-plane variography, validation against channels, boundary polygon, maximum distance, edits on the long section (`MIN_VN_VARIOGRAM`, `MIN_VN_ESTIMATE`, `MIN_VN_LONG`, `MIN_VN_EDIT`) |
| B3 — tonnage | planned | Dilution and recovery, blocks between levels and raises, categories by rules managed from MCP |
| B4 — branches | planned | Several veins, overlaps reported, incremental update when channels are added |

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
distance. Nodes with fewer than `min_samples` contacts within the search radius stay
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
