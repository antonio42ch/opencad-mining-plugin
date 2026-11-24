# Open Mining — user guide

[Español](es/GUIA_USUARIO.md) · [Install](INSTALL.md) · [Commands](COMMANDS.md) · [Narrow veins](VEINS.md)

How to use each module from the ribbon. Every button runs a command (`MIN_…`); the same commands can be
typed in the command line or sent by an AI assistant — see [AI_QUICKSTART.md](AI_QUICKSTART.md).
Data to practise with: [examples/](../examples).

## How it works

- **Tab and panels.** The ribbon has a **Mining** tab (*Minería*) with one group per module. A large button
  opens the module's **panel**; the small ones run an action with the panel's values.
- **Settings live in the drawing.** Paths, options and tables you set in a panel are saved in the DWG and come
  back when you reopen it.
- **One action, one undo.** Ctrl+Z undoes a whole drawing or calculation in one step. Redrawing replaces the
  previous drawing of the same thing: no duplicates.
- **Everything the plugin draws is tagged** (XDATA `OPENMINING`) so it can be queried, redrawn or removed.
- **Files next to the drawing.** Tables the plugin produces (converted channels, composites, prisms, blocks…)
  are written as `<drawing>_….csv`; save the drawing first.
- **Coordinates.** Data are in world coordinates. With a *local origin* enabled in the Drillholes panel, what
  is drawn is shifted by it (to keep numbers small); the plugin converts back when it reads the drawing.
- **Tables.** CSV, TXT, TSV, XLSX, XLS, ODS; `;` or `,` separators and decimal commas are recognised. Column
  names are matched by aliases (Spanish, English, Portuguese); if a column is not recognised a mapping dialog
  asks once and remembers it.
- **Languages.** Spanish, English, Portuguese; see [INSTALL.md](INSTALL.md#language).

## 1. Drillholes

Group **Drillholes** → **Panel**.

1. Choose the collar, survey, assays and (optional) geology tables → **Load**. The status line counts collars
   and validation messages (rules R01–R07: missing collars, duplicate holes, depth conflicts…).
2. Choose the grade to colour by (assay attribute) and the colour scheme (blue→red, rainbow, custom ranges) or
   a geology attribute with its colour table (**Build colour table** fills it from the data).
3. **Draw** → 3D traces (minimum-curvature desurvey) on one layer per assay attribute, with labels. Draw again
   and the previous drawing is replaced.
4. **Clear** removes everything the module drew.

Command line: `MIN_DH_LOAD collar=… survey=… assays=…`, `MIN_DH_DRAW assay_attr=au`, `MIN_DH_LIST hole=…`.

## 2. Channel samples

Group **Drillholes** → **Channels**. Systematic sampling inside the workings becomes **drillhole data**, so
all the drillhole tools (drawing, composites, estimation) and the vein model use it.

- **Topographic points** table: `POINT; EAST; NORTH; ELEVATION`.
- **Sampling sheet**, one row per sample: channel, topographic point, distance (m) from the point along the
  working, width of the sample (or from / to), and optionally the direction of the working (*towards* the next
  point or an azimuth), *potencia* (true vein thickness), sample type (`VETA`, `CAJA TECHO`…), working, sample
  id, position and **any number of grade columns** (`AU`, `AG`, `CU%`…). Detection limits such as `<0,01` are
  accepted.
- **Convert channels** → holes `CH-<channel>` are appended to the loaded drillholes. By default each channel
  is **horizontal and perpendicular** to the working, centred on its position. A channel can carry its own
  azimuth / inclination in the sheet, or be oriented on screen with **Orient channel** (two clicks); the
  orientation is remembered in a table.
- With *Reconvert on loading drillholes* the channels are appended again after every drillhole load.

Walk-through with data: [examples/canales](../examples/canales/README.md). Command line:
`MIN_CH_CONVERT points=… sheet=…`, `MIN_CH_ORIENT channel=… p1=… p2=…`.

## 3. Narrow veins

Group **Veins** (surfaces → prisms and grades → tonnage). A narrow vein is **not** modelled with a regular
block model: the plugin builds surfaces for the walls and 3D prisms between them. Full description with the
rules and every setting: **[VEINS.md](VEINS.md)**; worked example with the numbers to expect:
**[examples/veta](../examples/veta/README.md)**.

1. **Vein: surfaces** — hanging-wall and footwall from mapped contact polylines (by layer, or picked) and from
   the vein intervals of drillholes and channels. Check the diagnostics (crossings, worst contacts).
2. **Vein: prism model** — in-plane variogram of *grade × thickness*, prisms with kriged grade and thickness,
   validation against the samples, long section; draw a polygon on the long section to exclude prisms, set
   their grade or thickness, or limit the model.
3. **Vein: tonnage** — dilution (minimum mining width, overbreak with its grade) and recovery; blocks between
   levels and raises; sampled sides; **categories by rules** that you define in a table.
4. **Build all** runs the three stages in one step.

> The category rules that come in the table are **examples**, not a standard. Replace them with your
> company's norm and test them with `MIN_VN_RULES` before cubicating.

## 4. Workings, axes and sections

- **Axes**: start point (E, N, Z) and azimuth, then **Add straight** (length, grade in % or °) and **Add
  curve** (radius + angle or length, right / left); each element continues the same 3D polyline. **Remove
  last** and **New axis** edit the chain; **Continue polyline** recovers an axis from a saved drawing.
- **Workings**: **By axis** makes a closed solid (mesh) along an axis from a section (rectangle, horseshoe…);
  **H boundaries / V boundaries** make one from four polylines. The volume is printed in the command line.
- **Sections**: cut the solids with planes perpendicular to an axis, vertical through two points, horizontal
  levels or three points; 3D polylines in place and 2D sheets with grid, title and area.
- **Terrain in the sections**: choose a terrain — a Civil TIN surface of the drawing (`CREATESURFACE`) — in the
  *Terreno* section of the Sections panel (**Find terrains** refreshes the list). Each vertical section also
  shows the **terrain profile** and each level plan its **contour**, in green on `MIN_SC_TERRENO`, with the
  elevation range on every sheet. The terrain is drawn over the extent of the solids plus a margin (20 m by
  default; in vertical sections only along the section, never in height); with no solids selected, only the
  terrain is cut. Example: [examples/terreno](../examples/terreno/README.md).

## 5. Geological mapping

Group **Geology**. **Trace**: E / N / Z + bearing (`N45E`, `S30W`, or a plain azimuth) + length and anchor
draw a line on `MIN_GM_TRAZA` (the optional rock / thickness / grade fields are stored in its XDATA).
**Trace by click** previews it live. **Project** moves selected traces to another height along their dip.

## 6. Resource estimation

Group **Resources** — details and file formats in [RESOURCES.md](RESOURCES.md).

1. **Composites** panel: define *domains* (name, geology values, density, top-cut, optional contact surface or
   solids) → **Composite and analyse**: statistics and histograms per domain; `<drawing>_composites.csv`.
2. **Variography** panel: pick a domain, compute variograms along the three axes, **Auto-fit**, save the model
   in the domain, cross-validate.
3. **Model** panel: block size, rotation and search → **Estimate** (ordinary / simple kriging, IDW or nearest
   neighbour) → resources by domain; the block model is saved as `<drawing>.ombm`.
4. **Validate** (swath, grade-tonnage curve, histogram) and **Draw view** (levels, sections by two points or
   along an axis, cubes above a cut-off).

## 7. Geotechnics

Group **Geotechnics** — formulas and tables in [GEOTECH.md](GEOTECH.md); data to practise:
[examples/geotechnics](../examples/geotechnics/README.md).

- **Data**: load the geotechnical logging, laboratory and mapping tables → **Classify and draw**: RMR89, Q and
  GSI per interval, coloured by class. Missing data are reported, never filled in silently.
- **Hoek-Brown**: GSI, *mi*, D, σci (or **Use classified data**) → strength envelope and rock-mass modulus.
- **Model**: estimate the geotechnical attributes in the block model (with their confidence columns) and
  **View model**: by class, with a threshold filter and a plane cut, as plans, sections or cubes.
- **Structures and design**: stereonets and discontinuity sets, kinematic analysis, wedges; in-situ stress,
  Kirsch, convergence–confinement with support, pillars and crown pillars.

## Ask an AI assistant

With the assistant connected (see [AI_QUICKSTART.md](AI_QUICKSTART.md)), you can ask in plain words, e.g.:

- "Load the drillholes from this folder and tell me if validation found problems."
- "Convert the channel sheet and the topographic points, then show me the grades along the 3950 level."
- "Build vein V1 from the channels, check the diagnostics and tell me what you would change."
- "Why is block L1-L2/>R1 not *Medido*?"  (the assistant reads the block's `why` and the rule table)
- "Raise the minimum mining width to 1.2 m and compare the tonnes with the current model."

The assistant uses the same operations as the buttons, so you can see and undo everything it does.
