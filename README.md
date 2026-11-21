# Open Mining

Mining and geology add-on for [Open CAD Studio](https://github.com/HakanSeven12/OpenCADStudio)
(out-of-process `cdylib`, plugin API 23). Rust rewrite of PST_Sondajes,
PST_SolidMine and GeoMapper.

| Module | Commands | What it does |
|---|---|---|
| Drillholes | `MIN_DH_PANEL` `MIN_DH_LOAD` `MIN_DH_DRAW` `MIN_DH_CLEAR` `MIN_DH_GEOCOLORS` | CSV/XLSX import with column mapping, validation R01–R07, minimum-curvature desurvey, 3D traces coloured by grade or geology, labels |
| Channels | `MIN_CH_PANEL` `MIN_CH_CONVERT` `MIN_CH_ORIENT` | Channel samples of the workings, located from topographic points and distances, converted into drillhole data (holes `CH-…`) appended to the drillholes — see [examples/canales](examples/canales/README.md) |
| Veins | `MIN_VN_PANEL` `MIN_VN_SURFACES` `MIN_VN_MODEL_PANEL` `MIN_VN_VARIOGRAM` `MIN_VN_ESTIMATE` `MIN_VN_LONG` `MIN_VN_EDIT` | Narrow-vein model built from data: hanging-wall and footwall surfaces with diagnostics; 3D prisms between them with grades by accumulation kriged on the vein plane, validation against the channels, long section with polygon edits (dilution and tonnage by levels next) — see [docs/VEINS.md](docs/VEINS.md) |
| Workings | `MIN_UG_PANEL` `MIN_UG_AXIS` `MIN_UG_HBOUND` `MIN_UG_VBOUND` | Closed solids (mesh) from an axis + section, or from four boundary polylines |
| Axes | `MIN_AX_PANEL` `MIN_AX_LINE` `MIN_AX_CURVE` `MIN_AX_UNDO` `MIN_AX_NEW` `MIN_AX_PICKSTART` `MIN_AX_FROMPOLY` | Working axes as chains of straights and curves (radius + length or angle, right/left) with grade in % or °, drawn in series from the previous end |
| Sections | `MIN_SC_PANEL` `MIN_SC_RUN` | Cut solids (meshes) with planes perpendicular to an axis, vertical through 2 points (+ parallels), horizontal levels or 3 points; 3D polylines in place and 2D sheets with grid, title and area |
| Resources | `MIN_RS_COMP_PANEL` `MIN_RS_VARIO_PANEL` `MIN_RS_BM_PANEL` `MIN_RS_COMPOSITE` `MIN_RS_ESTIMATE` `MIN_RS_BMVIEW` … | Composites, domains (attribute, solids, contact surface), declustering, variography with auto-fit, ordinary/simple kriging, IDW and nearest neighbour on a rotated block model, validation (swath, grade-tonnage, cross-validation) and views — see [docs/RESOURCES.md](docs/RESOURCES.md) |
| Geotechnics | `MIN_GT_DATA_PANEL` `MIN_GT_HB_PANEL` `MIN_GT_MODEL_PANEL` `MIN_GT_LOAD` `MIN_GT_CLASSIFY` `MIN_GT_HB` `MIN_GT_ESTIMATE` `MIN_GT_VIEW` | Logging, face mapping and laboratory tables, RMR89 / Q / GSI classification, Hoek-Brown strength and modulus, geotechnical block model and views — see [docs/GEOTECH.md](docs/GEOTECH.md) |
| Structures & design | `MIN_RM_STRUCT_PANEL` `MIN_RM_DESIGN_PANEL` `MIN_ST_*` `MIN_SM_*` | Stereonets, discontinuity sets, kinematic analysis and wedges in workings; in situ stress, Kirsch, convergence-confinement with support, pillars and crown pillars — see [docs/GEOTECH.md](docs/GEOTECH.md) |
| Geology | `MIN_GM_PANEL` `MIN_GM_TRACE` `MIN_GM_TRACE_PICK` `MIN_GM_PROJECT` | Structure/vein/contact traces from coordinates + bearing/azimuth + length, and projection of mapped traces to another height along their dip |

**AI clients (MCP).** Every action is a typed operation with `key=value` arguments and a
machine-readable result line, so an assistant connected to `OpenCADStudio --mcp` can drive
the whole plugin with the `run` tool and read the outcome — see [docs/MCP.md](docs/MCP.md)
(generated from the definitions, with typical workflows). Read-only operations report the
project status, drillholes, statistics, block-model values and the plugin's entities.

UI in Spanish, English and Portuguese (`OPENMINING_LANG=es|en|pt`, else the
system locale).

## Install

Open CAD Studio → **Plugin Manager → Add repository** → `antonio42ch/opencad-mining-plugin`,
pick a release, **Install**, restart. Or copy the platform binary and
`plugin.toml` from a release into the user plugins folder.

## Develop

```bash
cargo test --workspace                       # all engines + plugin logic (mock CAD)
cargo clippy --workspace --all-targets -- -D warnings
cargo build -p opencad_mining --release      # target/release/libopencad_mining.{so,dll,dylib}
cargo bench -p drillhole_engine              # 1 000 holes: validate ≈ 45 ms, desurvey+map ≈ 80 ms
cargo bench -p blockmodel_engine             # 165 k-block OK estimation ≈ 0.4 s
cargo run -p openmining_cli -- desurvey --collar collar.csv --survey survey.csv
cargo run -p openmining_cli -- mcp-doc > docs/MCP.md   # regenerate the operations guide
cargo run -p openmining_cli -- ops-schema gt_hb      # JSON Schema of an operation
```

Release: bump `version` in `Cargo.toml` and `plugin.toml`, add a
`CHANGELOG.md` entry, tag `vX.Y.Z`. CI builds Linux/Windows/macOS assets.

## Documentation

- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) — layers, crates, data flow, design rules
- [docs/RESOURCES.md](docs/RESOURCES.md) — resource estimation: methods, conventions, file format
- [docs/MCP.md](docs/MCP.md) — every operation for AI clients: arguments, results, error codes, workflows
- [docs/VEINS.md](docs/VEINS.md) — narrow veins: channels, surfaces (and the coming prism model)
- [docs/GEOTECH.md](docs/GEOTECH.md) — geotechnics: tables, RMR89 / Q / GSI, Hoek-Brown, block model
- [docs/PLUGIN.md](docs/PLUGIN.md) — commands, panels, layers, **XDATA schema**, settings storage
- [docs/MIGRATION.md](docs/MIGRATION.md) — mapping from the legacy C# (AutoCAD) tools
- [docs/TESTING.md](docs/TESTING.md) — test locally (WSL/WSLg or Windows) before releasing
- [CHANGELOG.md](CHANGELOG.md)

## License

GPL-3.0-only (the host contract `ocs_plugin_api` is GPL-3.0-only).
