# Geotechnics (phases 1, 3 and 4)

Data, rock mass classification, Hoek-Brown strength and a geotechnical block
model. Empirical and analytical methods only: every value can be checked by
hand against the cited source. Underground mining first; Hoek-Brown also
serves slopes.

Engine: `crates/geotech_engine` (std only). Plugin: `geotech.rs` (logic,
tested with the mock CAD), `gtcommands.rs` (host), `gtpanels.rs`,
`gtconfig.rs`, `gti18n.rs`.

## Commands

| Command | Action |
|---|---|
| `MIN_GT_DATA_PANEL` / `MIN_GT_HB_PANEL` / `MIN_GT_MODEL_PANEL` | Show/hide the panels |
| `MIN_GT_LOAD [logging [mapping [lab [joints]]]]` | Read the tables (CSV, TXT, TSV, XLSX, XLS, ODS). Paths as arguments (quoted, `-` skips one) replace the panel's |
| `MIN_GT_CLASSIFY` | Classify every logging and mapping record, report and draw them. Mapping by chainage asks for the axis polylines (or uses the selection) |
| `MIN_GT_CLEAR` | Remove the drawn records |
| `MIN_GT_HB` | Hoek-Brown parameters, strength, Mohr-Coulomb fit, rock mass modulus and the envelope chart |
| `MIN_GT_HB_DATA` | Fill GSI with the median of the classified records (of the lithology typed, if any) and σci / mi / Ei from its laboratory row, then compute |
| `MIN_GT_ESTIMATE` | Interpolate RMR, GSI and log Q into the block model |
| `MIN_GT_VIEW` | Block model view (Resources view settings) coloured by the chosen geotechnical attribute |
| `MIN_GT_CLASSIFY_EXEC` | Internal follow-up of the axis pick |

After reopening a drawing the tables are read again from the paths saved in
the project (the column choices are remembered).

## Tables

Columns are recognised by name (Spanish, English, Portuguese; case, spaces,
`-`, `_` and `.` ignored). A table missing a required column opens a column
chooser once; the choice is saved with the project. Decimal comma and `;`
(Excel in Spanish) are read as in the drillhole importer.

| Table | Required | Optional |
|---|---|---|
| Logging | hole, from, to | everything in *Parameters* |
| Mapping (faces / cells) | X + Y, **or** chainage (or from/to chainages) | Z, axis, everything in *Parameters* |
| Laboratory | lithology | UCS, Is50, mi, Ei, Poisson, density |
| Discontinuities | dip, dip direction | hole, from, X, Y, Z, persistence, aperture, roughness, infilling (loaded now, analysed in phase 3) |

**Parameters**: lithology, RQD, fractures/m, spacing (m), UCS (MPa), Is50
(MPa), persistence (m), aperture (mm), roughness, infilling, weathering,
water, the five RMR ratings (`R1`…`R5` or `r_strength`, `r_rqd`, `r_spacing`,
`r_condition`, `r_water`), RMR, Jn, Jr, Ja, Jw, SRF, Q, GSI.

Roughness, weathering and water accept words (es/en) or the rating itself;
infilling accepts `ninguno`/`none` or its rating (hard < 5 mm 4, hard > 5 mm
2, soft < 5 mm 2, soft > 5 mm 0).

Mapping by chainage uses the **horizontal** chainage of the axis polyline (as
Labores and Secciones). With several axes selected, the *Axis* column is
compared with the layer of each polyline.

Drillhole logging is located on the holes loaded in Sondajes (minimum
curvature desurvey).

## Classification

**RMR89** (Bieniawski 1989, Table 4.1), ratings by class from the official
table — not from the continuous charts, which are graphs and are not
digitised here. Whoever reads the charts can type the ratings directly
(`R1`…`R5`), which always win over the measurements.

- Strength: UCS, else Is50, else the laboratory UCS / Is50 of the
  lithology.
- RQD: logged, else (optional, on by default) Priest & Hudson (1976)
  `RQD = 100 e^(−0.1λ)(0.1λ + 1)` from fractures/m.
- Spacing: logged, else `1 / fractures per metre`.
- Condition: the total rating, or all five sub-ratings (persistence,
  aperture, roughness, infilling, weathering). If one is missing it is named.
- No orientation adjustment in the basic value (it belongs to support
  design, where the orientation of the working is known).
- Class I–V: 100–81, 80–61, 60–41, 40–21, < 21.
- RMR14 will be added when its official tables (Celada et al. 2014) are
  available to verify against.

**Q-system** (Barton et al. 1974; NGI 2015): `Q' = (RQD/Jn)(Jr/Ja)`,
`Q = Q' · Jw / SRF`; RQD below 10 is taken as 10; parameters outside the
tables are refused. Q needs Jw and SRF; without them only Q' is computed.

**GSI**: typed, else `RMR89' − 5` (RMR89' = dry, no orientation; Hoek et al.
1995, valid for RMR89' > 23), else `9 ln Q' + 44`.

**RMR ↔ Q** (only when *Completar* is ticked): Bieniawski `9 ln Q + 44`,
Rutledge & Preston `5.9 ln Q + 43`, or Barton `15 log Q + 50`.

**Nothing is invented.** A record lacking a parameter keeps that value empty;
the report counts the records missing each parameter, lists values outside
the tables (row and column), counts the values not measured directly
(laboratory UCS, RQD from fractures, correlations…) and the records that
could not be located, with the reason.

## Hoek-Brown

Generalised criterion, 2002 edition (Hoek, Carranza-Torres & Corkum):

```
mb = mi exp((GSI − 100) / (28 − 14D))
s  = exp((GSI − 100) / (9 − 3D))
a  = 1/2 + (e^(−GSI/15) − e^(−20/3)) / 6
σ1 = σ3 + σci (mb σ3/σci + s)^a
```

Reported: σc of the mass (`σci s^a`), σt (`−s σci / mb`), global strength
σcm, equivalent Mohr-Coulomb c' and φ' over `0 < σ3 < σ3max` with
`σ3max/σcm = 0.47 (σcm/γH)^−0.94` (tunnels) or `0.72 (σcm/γH)^−0.91`
(slopes), and the rock mass modulus of Hoek & Diederichs (2006): generalised
with Ei, simplified without it. The chart shows the HB envelope and the MC
line over the same range.

## Geotechnical block model

Uses the block model of the tab (Resources) or of `<drawing>.ombm`; without
one it creates a model covering the records with the Resources geometry
(size, rotation, pad) and a single *Global* domain. Inverse distance or
nearest neighbour, search radii (radius 1 along the azimuth), minimum /
maximum samples and maximum per hole. With *Respetar dominios* each record
goes only to the domain whose values list its lithology.

Columns written: `gt_rmr`, `gt_gsi`, `gt_logq` (Q is interpolated as
log10 Q because it spans orders of magnitude) and `gt_class` (from `gt_rmr`).
Density and tonnage of the resource model are not touched. The model is saved
to `<drawing>.ombm`.

`MIN_GT_VIEW` has its own view settings in the *Modelo* panel: **mode** (level
plans, sections by two points, sections along an axis, or **cubes**), levels
(from / to / step), section spacing, count and interval, and the maximum number
of polygons or cubes. Cubes draw one mesh per colour with every block that has a
value (any domain, no cut-off); above the limit they are an even sample and the
message says how many were drawn. Class uses the class colours; RMR, GSI and
log Q use a blue-to-red scale with **poor rock in red**. Sections need
`p1=`/`p2=` (two points) or `axis=` (a polyline handle); without them the pick
starts. The settings are also copied into the Resources view settings.

## Drawing

| Layer | Content |
|---|---|
| `MIN_GT_SONDAJE` | logged intervals (3D polylines along the hole) |
| `MIN_GT_MAPEO` | mapped stretches (polylines along the axis) or points, and labels `RMR · Q · GSI` |

Colour by RMR class (I blue, II green, III yellow, IV orange, V red), Q class
(> 40 blue, 10–40 green, 4–10 yellow, 1–4 orange, 0.1–1 red, < 0.1 magenta)
or GSI in 20-point bins. Unclassified records are grey.

XDATA class `GT_REC` (*Registro geotécnico*): `src` (`logging`/`mapping`),
`row`, `id` (hole or axis), `rmr`?, `q`?, `qp`?, `gsi`?, `class`?, `lith`?,
`label` (on label texts).

## Verification

Each formula has unit tests against the published values (class limits and
boundaries of every RMR89 table, the Priest & Hudson value at 6 fractures/m,
Q examples, the Hoek et al. 2002 parameters for GSI 50 / mi 10 / D 0:
mb 1.677, s 0.0039, a 0.506, the simplified modulus at GSI 50). The sample
tables in `crates/geotech_engine/tests/data` are classified by the CI test
and by the end-to-end test in the real host.

# Phase 3 — Structures

Panel **Geotecnia: estructuras**. Poles come from the discontinuity table loaded
with `gt_load joints=...` (dip, dip direction) and, optionally, from geological
traces that store a dip (`gm_trace dip=...`).

| Operation | Command | What it does |
|---|---|---|
| `st_stereonet` | `MIN_ST_STEREONET` | Equal-area (Schmidt) lower-hemisphere stereonet: poles, density contours (1 % counting cone, half-angle 8.11°; antipodal poles count together), set great circles |
| `st_sets` | `MIN_ST_SETS` | Sets by spherical clustering: seeds at density peaks ≥ `min_angle` apart, k-means on axes (orientation-tensor mean), poles beyond `max_angle` unassigned. Fisher K = (N−1)/(N−R) and α95. Writes the editable sets table (φ and c kept) |
| `st_kinematic` | `MIN_ST_KINEMATIC` | Markland test (Hoek & Bray 1981): planar sliding (φ < dip < face dip, within ±lateral), wedge sliding (intersection plunge between φ and the apparent face dip in its trend), flexural toppling (Goodman 1980: dips into the face, dip ≥ 90 − face dip + φ). **Direct toppling is not included** until its criterion is verified |
| `st_wedges` | `MIN_ST_WEDGES` | Largest wedge of three sets on the roof and each wall (planar faces, Hoek & Brown 1980): trace scaled to the span (and `max_length`), apex height, volume, weight; stability by the vector method of Hoek & Bray: falling, sliding on one plane, sliding on two planes, with φ, c per set and a support pressure on the base. Drawn on the working at `chainage` of an axis polyline (`axis=`) or at a point (`at=`) |
| `st_draw` | `MIN_ST_DRAW` | The stereonet as 2D entities (layer `MIN_ST_ESTEREO`) |

Conventions: x east, y north, z up; dip 0–90, dip direction clockwise from
north; pole = downward normal; projection radius `√2·sin((90 − plunge)/2)`.
Two sets whose intersection is parallel to a face close no wedge on it (reported).

# Phase 4 — Stresses and design

Panel **Geotecnia: esfuerzos y diseño**. The rock mass strength comes from the
Hoek-Brown panel (σci, GSI, mi, D, Ei).

| Operation | Command | What it does |
|---|---|---|
| `sm_stress` | `MIN_SM_STRESS` | σv = γ·H/1000 (MPa), σH = KH·σv at its azimuth, σh = Kh·σv; components in the section of a working with the given trend (σx across, σz vertical, τ, in-plane σ1, σ3 and angle, axial) |
| `sm_kirsch` | `MIN_SM_KIRSCH` | Kirsch (1898) around a circular opening in the in-plane σ1, σ3: σθ on the boundary (3σ1 − σ3 at the sides), strength factor σc(mass)/σθmax, depth of the zone where σθ exceeds the Hoek-Brown strength at σr. **Non-circular openings use the equal-area radius (approximation)** |
| `sm_convergence` | `MIN_SM_CONVERGENCE` | Convergence-confinement (Hoek 1998): ground curve of Duncan Fama (1993) with c′, φ′ of the Hoek-Brown fit for tunnels (Hoek et al. 2002) and Erm (Hoek & Diederichs 2006, or typed); p0 = mean in-plane stress (the method assumes a hydrostatic field); displacement profile of Vlachopoulos & Diederichs (2009) for the installation distance; shotcrete and mechanically anchored bolts (Hoek 1998; stiffnesses add up to the smallest yield displacement); equilibrium, support FS, strain and Hoek & Marinos (2000) squeezing class |
| `sm_pillar` | `MIN_SM_PILLAR` | Tributary-area stress (square, rectangular, rib), extraction, strength by Lunder & Pakalnis (1997), Obert & Duvall (1967), Bieniawski (1975) or Hedley & Grant (1972, K = 133·UCS/230); effective width 4A/P for rectangular pillars; FS of each formula |
| `sm_span` | `MIN_SM_SPAN` | Crown pillar scaled span Cs = S·[γ/(t(1+S/L)(1−0.4 cos θ))]^0.5 (Carter 1992), compared with **the critical scaled span you give** (the Sc–Q relation and Lang's span chart are empirical and are not built in) |
| `sm_draw` | `MIN_SM_DRAW` | The last Kirsch or convergence chart as polylines (layer `MIN_SM_GRAFICO`) |

Fixed values: shotcrete Poisson 0.25, bolt steel modulus 200 GPa. Bolt capacity
and the deformation-load constant Q are inputs (use the bolt's data).

## Drawing (phases 3–4)

| Layer | Content |
|---|---|
| `MIN_ST_ESTEREO` | stereonet (net, poles, contours, sets) |
| `MIN_ST_CUNAS` | wedges (meshes; red FS < 1, orange < 1.5, green otherwise) |
| `MIN_SM_GRAFICO` | design charts |

XDATA: `RM_WEDGE` (*Cuña*): `face`, `mode`, `fs`?, `volume`, `weight`, `sets`;
`RM_DRAW` (*Dibujo geomecánico*): `kind` (`stereonet`, `chart_kirsch`,
`chart_convergence`), `series`?.

## Verification (phases 3–4)

Unit tests against closed forms and published values: projection reference
points and inverse; pole and intersection geometry; Fisher statistics; three
simulated sets recovered (and steep sets across the rim); Markland cases; a block
on one plane gives tan φ / tan ψ; a symmetric two-plane wedge matches Hoek &
Bray's wedge factor; roof pyramid falling and FS = support / weight; Kirsch 3σ1 −
σ3, hydrostatic 2p and the far field; Duncan Fama elastic limit, continuity at
pcr; LDP limits; shotcrete pressure; Lunder-Pakalnis ≈ 0.42 UCS at w/h = 1;
tributary-area loads; Carter's formula. The sample discontinuities
(`crates/geotech_engine/tests/data/joints.csv`, 135 poles in three sets) are
grouped back into their sets by a CI test and by the end-to-end host test.
