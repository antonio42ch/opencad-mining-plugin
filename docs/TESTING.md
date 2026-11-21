# Testing before a release

The plugin is found in `<config>/OpenCADStudio/plugins/opencad.mining/`
(`plugin.toml` + one library). `OCS_PLUGINS_DIR` overrides the folder. The
host must be a **release with the same plugin API** as `plugin.toml`
(currently 23 = host v2026.46).

## A. Linux host inside WSL (WSLg) — fastest loop

```bash
# once: download the host and extract it (no FUSE needed)
mkdir -p ~/ocs && cd ~/ocs
curl -LO https://github.com/HakanSeven12/OpenCADStudio/releases/download/v2026.46/OpenCADStudio-v2026.46-linux-x86_64.AppImage
chmod +x OpenCADStudio-*.AppImage && ./OpenCADStudio-*.AppImage --appimage-extract

# every change
scripts/dev-install.sh                       # build + copy to ~/.config/OpenCADStudio/plugins
~/ocs/squashfs-root/AppRun                   # opens in a Windows window through WSLg
```

## B. Windows host (what most users run)

```bash
sudo apt install -y mingw-w64                # once
scripts/dev-install.sh --windows             # .dll -> %APPDATA%\OpenCADStudio\plugins
```

Close Open CAD Studio before installing (Windows locks the loaded `.dll`),
then start it again. The release CI builds the official `.dll` with MSVC; if
the MinGW build misbehaves, build on Windows itself with
`cargo build -p opencad_mining --release` (Rust for Windows) and copy
`target\release\opencad_mining.dll` + `plugin.toml` to the same folder.

## C. Automated smoke test (headless)

```bash
python3 scripts/smoke_serve.py ~/ocs/squashfs-root/AppRun
```

Uses a temporary config and plugins folder and drives every module over
`--serve` **the way an AI client does** — `key=value` arguments, handles instead of
clicks, and the `OPENMINING_RESULT` line of every command checked — plus one value typed
into a generated panel: plugin loaded and its 11 panels registered, status queries, geology trace from
coordinates and projection along the dip, axis solid and level sections,
drillholes from the reference CSVs (typed into the panel), composites, a saved
variogram model, block model estimation and a block view, then geotechnics
(sample logging + laboratory tables classified on the loaded holes, Hoek-Brown,
geotechnical block model columns and a view coloured by RMR). On failure it prints
every request/reply and the host's command line. CI runs it on each push
against the host release pinned in `.github/workflows/ci.yml` (`OCS_VERSION`).

## Manual checklist (GUI)

1. Ribbon shows the **Minería** tab (Sondajes / Labores / Geología).
2. Sondajes panel: pick `crates/drillhole_engine/tests/data/{collar,survey,assays}.csv`,
   **Cargar y validar** → status line counts 3 collars, 0 messages.
3. **Dibujar** → 3 traces, labels, coloured assays; **Dibujar** again → same
   entity count (no duplicates). Ctrl+Z undoes the whole draw in one step.
4. Properties of a trace show *Traza de sondaje (Polyline3D)*.
5. Rename a column in `collar.csv` → load → the mapping dialog appears; load
   again → it is not asked again.
6. Save as DWG, close, reopen → the panel paths/options come back
   ("configuración restaurada"); **Dibujar** works after **Cargar**.
7. Labores: draw a `3DPOLY`, select it, **Por eje** → solid on `MIN_UG_SOLIDO`
   with its volume on the command line.
8. Geología: **Traza** → live red preview, click → line + texts.
9. Plugin Manager lists *Open Mining* as enabled with no errors.
10. Ejes: start E/N/Z + azimuth, grade −12 %, **Agregar recta** 50 m, then
    **Agregar curva** R 15 m, 90°, Derecha → one magenta polyline; the panel
    start moved to its end. **Quitar último** removes the curve.
11. Close and reopen the DWG, **Continuar polilínea** on the axis → message
    "Eje … recuperado"; new elements extend the same polyline.
12. Labores → **Por eje** on that axis, then Secciones → mode
    *Perpendiculares a un eje*, interval 10 → select the solid, pick the axis →
    red 3D sections on `MIN_SC_CORTE` and sheets to the right with PK titles
    and areas; run again → no duplicates.
13. Secciones *Horizontales* with Cota final ≤ Cota inicial → levels over the
    whole solid automatically.

Logs: the host writes plugin load errors (API/wire-schema mismatch, runner
crashes) to the command line and to `automation/gui.log` in the config folder.

## Resource estimation (v0.4)

Use the drillhole test data or your own (collar, survey, assays with an
attribute such as `AU`, geology with e.g. `ROCK_TYPE` = OXIDO/SULFURO).

14. Sondajes → **Cargar**, choose the grade attribute (Colores) and the geology attribute.
15. Recursos → **Compósitos**: fill the *Dominios* table (name, geology values separated by `;`,
    density, top-cut, surface *Encima*/*Debajo*), **Compositar y analizar** → statistics on the
    command line and in the panel, a histogram per domain, `<drawing>_composites.csv` next to the DWG;
    **Dibujar compósitos** twice → no duplicates.
16. **Calcular superficie de contacto** (geology above/below) → a surface mesh on `MIN_RS_CONTACTO`.
    Optionally select wireframe meshes and **Asignar sólidos seleccionados** to a domain.
17. Recursos → **Variografía**: pick a domain, **Calcular variogramas** → chart with the 3 axes
    (+ omni); **Autoajustar** → fields change and the curve follows; **Guardar en el dominio**;
    **Validación cruzada** → statistics line. The panel must stay open after each chart refresh.
18. Recursos → **Modelo**: **Estimar** → block/estimation lines and resources by domain on the
    command line; `<drawing>.ombm` appears next to the DWG (save the drawing first).
19. **Validar** with each chart type (swath / tonnage curve / histogram).
20. **Dibujar vista**: *Niveles*, *Secciones por 2 puntos* (pick 2 points), *Secciones perpendiculares
    a un eje* (pick a polyline), *Cubos sobre la ley de corte*; redraw → the previous view is replaced.
21. Close and reopen the DWG: the panels come back configured; **Cargar archivo** restores the model.

## Geological mapping (v0.5)

22. Geología → panel: type E / N / Z, a bearing (`N45E`) and a length, **Dibujar traza** → one red line
    on `MIN_GM_TRAZA` at those coordinates and no texts. **Trazar con clic** → live preview, and the
    clicked point fills E / N / Z.
23. Draw a line or polyline at some elevation, type a dip (e.g. 60) and a direction, set *Altura de
    destino* below it and **Proyectar trazas seleccionadas** (selected first, or pick + Enter) → an
    orange copy on `MIN_GM_PROYECCION` at that height, displaced down-dip by `Δz / tan(dip)`.
    Check the side (right/left of the drawing direction), then *Diferencia de altura* with a positive and a
    negative value (the line moves against / with the dip). Reverse the polyline and the side flips.
24. Draw a trace with a dip, leave the dip field empty, project it → uses the stored dip.

## Geotechnics (v0.6)

25. Geotecnia → Datos: choose `crates/geotech_engine/tests/data/logging.csv` as logging and
    `lab.csv` as laboratory (after loading the reference drillholes), **Cargar** → two lines with
    the records and recognised columns. **Clasificar y dibujar** → 5 coloured intervals on
    `MIN_GT_SONDAJE` along VERT and INCL; the report gives RMR / Q / GSI counts and the classes
    (VERT 0–20 m is RMR 84, class I, blue; VERT 40–60 m is RMR 36, class IV, orange).
26. Delete the UCS of a Brecha row in a copy of the file and reload → that record is grey and the
    report counts it under *UCS* (nothing is filled in silently).
27. Mapping by chainage: a table with `PK;RMR;Q` rows, draw an axis, **Clasificar** → asks for the
    axis; points with labels appear at those horizontal chainages on `MIN_GT_MAPEO`.
28. Hoek-Brown: GSI 50, mi 10, D 0, σci 100 → mb 1.677, s 0.00387, a 0.506 and a chart with the
    curve and the dashed Mohr-Coulomb line. Type `Andesita` as lithology, **Usar datos
    clasificados** → σci 140, mi 25 and the median GSI.
29. Modelo: **Estimar geotecnia** (with or without a resource model) → `gt_rmr`, `gt_gsi`, `gt_logq`,
    `gt_class` reported; **Ver modelo** with *Clase de macizo* → blocks in the class colours.
    The resource tonnage report is unchanged afterwards.

## Structures, stresses and design (v0.8)

30. Geotecnia → Datos: discontinuidades `examples/geotechnics/discontinuidades.csv` → **Cargar**
    (135 records). Estructuras → **Estereograma** → poles and contours in the panel.
31. **Calcular familias** → three sets near 78/040, 62/150 and 14/265 in the table; edit φ of
    one row, run again: φ is kept.
32. Cinemática: cara 70 / 040, φ 30 → planar, wedge and toppling percentages and the critical
    sets; the face appears dashed on the stereonet.
33. Cuñas: sets 1, 2, 3, width 4, height 4, draw an axis and run `MIN_ST_WEDGES axis=<handle>
    chainage=10` → coloured wedges on the roof and walls at chainage 10, with mode, FS and volume.
    Raise the support pressure: the falling wedge's FS grows (support / weight).
34. Esfuerzos y diseño: depth 500, K 1.5 / 1.0, σH azimuth 90, axis trend 0 → σv 13.5 MPa and
    σ1 20.25 MPa across the working; Kirsch with radius 2.5 → σθ max 47.25 MPa and the chart.
35. Convergencia with shotcrete 0.1 m installed 2 m from the face → ground and support curves,
    equilibrium and FS (or "the support reaches its capacity"); squeezing class.
36. Pilares 8 × 8 × 5 m, rooms 6 m → load, four strengths and FS; pilar corona with a critical
    Sc → Sc / Cs. **Dibujar gráfico** draws the last chart.
37. Geotecnia → Modelo → **Estimar geotecnia**, then *Tipo de vista* = **Cubos**, *Atributo* =
    Clase de macizo, **Ver modelo** → one cube per block coloured by class (blue to red), one mesh per
    colour on `MIN_RS_VISTA`. Set *Máximo de polígonos / cubos* to 20 → the command line says how many
    of how many blocks were drawn. Switch back to *Niveles (planta)* → plans again.
38. Modelo → **Estimar geotecnia**, then in *Vista* choose *Atributo* = Clase de macizo, *Tipo de vista* = Cubos and open
    *Filtro y corte*: *Mostrar* = solo mayores que el umbral, umbral 3, **Ver modelo** → only classes IV and V.
    Choose *Atributo* = Distancia media a los registros → the blocks far from any record are red.
39. *Cortar con un plano*: a vertical plane (buzamiento 90, dirección 90) through the middle of the model, side
    *El lado hacia el que apunta el plano* → half the model; the other side → the rest; together every block.
40. `MIN_HELP` lists the operations; `MIN_HELP op=gt_view` gives the parameters, the options of `mode` and the schema;
    `MIN_HELP workflows=true` adds the command sequences and the error codes.

## Channel samples (v0.10)

41. Load the reference drillholes, then Sondajes → **Canales**: `examples/canales/puntos_topograficos.csv` and
    `muestreo_canales.csv` → **Convertir canales** → "24 convertidos (72 muestras)". **Dibujar** with
    attribute `au`: short coloured channels on the backs of the two galleries, across the N30E vein.
42. Type channel `3950-101-03` → **Orientar canal en pantalla**, two clicks → that channel turns to the
    clicked direction and appears in the orientations table. Reload the drillholes (`MIN_DH_LOAD`): the
    channels come back by themselves ("Canales agregados de nuevo").
43. Delete the HACIA value of one row and the azimut column is absent → that channel is reported (no
    direction) and the rest are converted.

## Vein surfaces (v0.10)

44. Draw two 3D polylines on layer `VETA_TECHO` and two on `VETA_PISO` (the walls of a vein at two levels),
    Vetas → **Veta** → **Construir superficies** → plane orientation, contact counts, thickness range and
    RMS in the command line; orange (hanging wall) and cyan (footwall) meshes.
45. Move one footwall vertex across the hanging wall → crossings reported near that polyline, and it is the
    first "worst" contact. Reduce the radius → nodes with one wall only are reported.
46. With channels converted (step 41), tick *Usar intersecciones* (attribute `tipo`, value `VETA`) → the
    contact counts include the intercepts.

## Vein prism model (v0.10)

47. With the surfaces of step 44 and channels with `tipo = VETA` and `au`, Vetas → **Modelo** →
    *Variografía en el plano* → composites count, model in the settings and the chart.
48. **Estimar prismas** → coloured prisms between the walls on `MIN_VN_PRISMAS`, totals, validation lines,
    and `<dwg>_veta_V1_prismas.csv`. Change *Colorear por* to *Potencia* and estimate again.
49. **Longitudinal** → long section beside the model with the channel labels. Draw a closed polyline over part
    of it, *Edición* = *Excluir prismas*, **Editar en la longitudinal**, pick it → those prisms disappear in 3D and on
    the long section (magenta outline). *Polígono límite* with another polyline → only the prisms inside remain.
50. *Deshacer la última*, then *Borrar ediciones y límite* → the model returns to step 48.
