# Geotechnics (phase 1)

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

`MIN_GT_VIEW` draws with the Resources view settings (levels, sections along
two points or an axis, cubes). Class uses the class colours; RMR, GSI and
log Q use a blue-to-red scale with **poor rock in red**. The cubes view keeps
an even sample when there are more blocks than the limit.

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
