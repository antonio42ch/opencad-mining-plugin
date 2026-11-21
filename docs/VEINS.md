# Narrow veins

Narrow veins are not modelled with the regular block model. The vein model is
built from data in stages, each an operation an AI assistant can run, check and
adjust (see [MCP.md](MCP.md)):

| Stage | Status | What it does |
|---|---|---|
| A — channels | done | Channel samples of the workings converted into drillhole data (`MIN_CH_CONVERT`, see [examples/canales](../examples/canales/README.md)) |
| B1 — surfaces | done | Hanging-wall and footwall surfaces from mapped contacts and vein intercepts (`MIN_VN_SURFACES`) |
| B2 — prisms and grades | planned | 3D prisms between the surfaces, grades by accumulation on the vein plane, variography, validation against channels, boundary polygon, edits on the long section |
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
