# Resource estimation (Recursos) — reference

Workflow, inside one drawing:

```
Sondajes (load) ─► Compositar ─► Dominios ─► Variografía ─► Modelo de bloques ─► Validar ─► Vistas
                      │              ▲  ▲          │              │
                      │   sólidos ───┘  └─ superficie de contacto │ (krigeado 2D de los contactos)
                      └─ desagrupamiento, tope de ley            └─ recursos por dominio, archivo .ombm, CSV
```

Everything reuses the drillholes already loaded (`MIN_DH_LOAD`) and the grade /
geology attributes and colour scheme chosen in the **Sondajes** panel.

## Panels and commands

| Panel | Command | Action |
|---|---|---|
| Compósitos y dominios | `MIN_RS_COMP_PANEL` | Show/hide |
| | `MIN_RS_COMPOSITE` | Composite, top-cut and decluster per domain; statistics, histogram, CSV next to the drawing |
| | `MIN_RS_DRAWCOMP` | Draw composites as lines coloured by grade (replaces the previous drawing) |
| | `MIN_RS_SETSOLID` | Assign the selected solids (Mesh / PolyfaceMesh / 3DFACE) to the domain chosen in the panel |
| | `MIN_RS_CONTACT` | Estimate the contact surface (ordinary kriging of the contact elevation) and draw it |
| Variografía | `MIN_RS_VARIO_PANEL` | Show/hide |
| | `MIN_RS_VARIO` | Experimental variograms of the selected domain along the 3 principal axes (+ omnidirectional) |
| | `MIN_RS_VARIOUPDATE` | Redraw the chart with the current model |
| | `MIN_RS_FIT` | Automatic fit (weighted least squares, Cressie weights) of the working model |
| | `MIN_RS_MODELSAVE` / `MIN_RS_MODELLOAD` | Store / recall the working model of a domain |
| | `MIN_RS_CROSSVAL` | Leave-one-out cross-validation (other holes only) |
| Modelo de bloques | `MIN_RS_BM_PANEL` | Show/hide |
| | `MIN_RS_BMCREATE` | Create the model and assign blocks to domains (no estimation) |
| | `MIN_RS_ESTIMATE` | Create + estimate every domain, save `<drawing>.ombm`, print resources by domain |
| | `MIN_RS_VALIDATE` | Statistics and chart: swath plot / grade-tonnage curve / histogram |
| | `MIN_RS_BMVIEW` | Draw a view: levels, sections by 2 points, sections perpendicular to an axis, or cubes above a cut-off |
| | `MIN_RS_BMLOAD` / `MIN_RS_BMEXPORT` | Load the `.ombm` file / export `<drawing>_bloques.csv` |
| internal | `MIN_RS_SETSOLID_EXEC`, `MIN_RS_BMVIEW_EXEC` | Follow-ups of the interactive picks |

Chart refresh note: the host rebuilds a panel whose definition changes, so
after drawing a chart the panel is shown again and all fields are restored
from the saved settings.

## Methods

**Compositing.** Intervals are cut at the geology contacts (domain per piece
from the attribute values listed for each domain, case-insensitive). Each run of
one domain is composited from its first depth in pieces of the chosen length. The
grade is the length-weighted mean of the *sampled* parts (gaps do not dilute);
pieces with less sampled length than *cobertura mínima* are dropped. A short final
piece (< *fracción mínima* × length) is merged into the previous one — only when that
composite ends exactly where the piece starts (never across a dropped, unsampled
stretch) and the merged composite still meets the coverage — or discarded or kept.
Metal and total length are preserved (tested).

**Composites and the domain table.** A composite's domain is its position in the
domain table at compositing time. After editing the table (names, order), run
**Compositar** again: variography, cross-validation, estimation and validation refuse
stale composites with a message.

**Top-cut.** One value per domain; applied to composites before statistics,
declustering, variography and estimation.

**Declustering.** Cell declustering over `dc_steps` sizes between `dc_min`
and `dc_max` (vertical cell = size × ratio), weights averaged over 4 shifted
grids; the size with the **lowest** declustered mean is kept (over-sampled high
grades). The declustered mean feeds validation and the SK mean.

**Experimental variogram.** Sector method: azimuth tolerance, dip tolerance,
horizontal/vertical bandwidths (0 = none), lag and lag tolerance (= lag/2).
Pairs are symmetric. Directions are the three principal axes of the model
orientation. `γ(h) = 1/(2N) Σ (z_i − z_j)²`, `h` = mean pair distance of the bin.

**Model.** `γ(h) = C0 + Σ C_k · f_k(h_r)`, spherical / exponential / gaussian
(practical range; exponential and gaussian reach 95 % at the range), geometric
anisotropy with ranges along `[major, semi-major, minor]`. The UI edits two
structures; the engine supports three. *Orientation*: azimuth of the major axis
(clockwise from north), dip of the major axis **positive downwards**, rake rotates
the other two axes about the major. With dip = rake = 0 the semi-major axis is
horizontal and the minor axis vertical. (Match your software's convention before
comparing parameters: this is not GSLIB's angle triple.)

**Auto-fit.** Nelder–Mead on nugget, sills and ranges with objective
`Σ N_pairs · (γ_exp − γ_model)² / γ_model²`; the orientation stays as set. Without
experimental pairs the working model is left untouched. Cached experimental variograms
belong to one domain and are recomputed when the domain or the composites change.

**Cross-validation.** Each composite is re-estimated from composites of *other* holes;
those are excluded before the neighbourhood limits, so the search keeps its full size.

**Estimation.** Per domain, only composites of that domain. Block =
`nx × ny × nz` regular discretisation (default 4×4×4).
- *OK*: `Σλ_i γ(x_i, V̄) + μ − γ̄(V,V)` is the kriging variance; kriging efficiency
  `KE = (σ²_V − σ²_K)/σ²_V` with `σ²_V = C(0) − γ̄(V,V)`; slope is the
  approximation `(σ²_V−σ²_K+|μ|)/(σ²_V−σ²_K+2|μ|)`.
- *SK*: known mean per domain (default: declustered mean), covariance form.
- *IDW*: power configurable, distance to the block centre. *NN*: nearest composite.
- Search: ellipsoid with the **domain model's orientation**, radii `[major, semi,
  minor]`, min/max composites, max per octant (0 = no limit), max per hole.
  Up to 3 passes with growing radii; the pass that succeeded is stored
  (`<var>_pass`) — the basis of the resource category in the next phase.
- A tiny diagonal regularisation (1e-9 × sill) keeps duplicate samples from making
  the system singular.

**Block model.** Origin = lower corner, block size, plan rotation (azimuth of
the row axis; 0 = east/north aligned). Automatic extent covers the composites
(+ margin), snapped to multiples of the block size.

**Domains of blocks.** Per domain: its solids (union) and/or the side of the
contact surface (intersection). A block takes the first domain (in table order)
that contains most of its discretisation nodes (every node is tested unless the block
lies outside the bounding box of all regions, so solids smaller than a block are not
missed); `fraction` is the share inside it
(blocks under *proporción mínima* are dropped). One domain without solid or
surface covers the whole model.
`tonnage = block volume × fraction × density`.

## Files

`<drawing>.ombm` (binary, version 1; also `…_composites.csv`, `…_bloques.csv`),
next to the DWG or in the folder set in the panel. A drawing must be saved (or a
folder given) to write them.

```
OMBM1\n
geometry=ox,oy,oz,dx,dy,dz,nx,ny,nz,rotation\n
domains=name1|name2\n
columns=c1,c2,...\n
end\n
domain: n × u8 | fraction: n × f32 LE | each column in header order: n × f32 LE (NaN = no value)
```
Index of block `(i,j,k)` = `(k·ny + j)·nx + i`. Columns for variable `au`:
`au_ok, au_ok_var, au_ok_ke, au_ok_slope, au_sk, au_idw, au_nn, au_n, au_pass,
au_dist, density, tonnage`. The reader rejects truncated or oversized files, absurd block counts and domain ids
without a name.

## Settings and XDATA

Settings (all panel fields, saved models per domain, column maps…) live in the
`OPENMINING/PROJECT` xrecord with the rest of the project. XDATA classes:

| Class | Key | Fields |
|---|---|---|
| Composite line | `RS_COMP` | `hole`, `from`, `to`, `value`, `domain` |
| Contact surface (Mesh) | `RS_SURFACE` | `points`, `nx`, `ny`, `step` |
| Model view | `RS_VIEW` | `set`, `mode`, `plane`, `block` (polygons) / `blocks` (cubes) |
| Domain solid | `RS_DOMAIN` | under the second application `OPENMINING_DOM`: `domain` |

Layers: `MIN_RS_COMP_<VAR>`, `MIN_RS_CONTACTO`, `MIN_RS_VISTA`.

## Limits and next phase

- Domain solids must be meshes; ACIS `3DSOLID` are not read. Faces are tested as
  triangle fans (exact for convex/planar faces such as the plugin's solids).
- One contact surface serves all domains that use one.
- Views are limited to *Máximo de polígonos* (default 20 000) for drawing speed.
- **Not yet:** resource categorisation (kriging-variance cut-offs, emulated grids,
  Measured/Indicated/Inferred), simulation, reserves. The data they need
  (`_ok_var`, `_pass`, `_n`, `_dist`) is already in the model.
