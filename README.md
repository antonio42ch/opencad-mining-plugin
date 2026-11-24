# Open Mining

**English** · [Español](README.es.md)

Mining and geology add-on for [Open CAD Studio](https://github.com/HakanSeven12/OpenCADStudio):
drillholes and channel samples, underground workings, geological mapping, resource estimation,
geotechnics, and a **narrow-vein model built from the data** (surfaces, prisms, grades, dilution,
tonnage by blocks and categories). Everything can also be driven by an **AI assistant** through MCP.

Free software, GPL-3.0. Version **0.11.0** · plugin API 28 · Windows, Linux and macOS (Apple Silicon).
Panels and messages in Spanish, English and Portuguese.

## Install (5 steps, from this page)

1. Open **Open CAD Studio** and go to the **Plugin Manager**.
2. Choose **Add repository** and paste this link (or just `antonio42ch/opencad-mining-plugin`):

   ```
   https://github.com/antonio42ch/opencad-mining-plugin
   ```
3. Pick the release **v0.11.0** and press **Install**.
4. **Restart** Open CAD Studio.
5. A **Mining** tab (*Minería* in Spanish) appears in the ribbon. To check it works, type `MIN_STATUS`
   in the command line: it reports what is loaded and what to do next.

Problems? See [docs/INSTALL.md](docs/INSTALL.md) (manual installation, troubleshooting, updating).
Needs Open CAD Studio **v2026.47 or newer** (plugin API 28); for v2026.46 and older use release v0.10.0.

## What you can do

| | |
|---|---|
| **Drillholes** | Load collar / survey / assays / geology (CSV, XLSX), validate, desurvey, draw in 3D coloured by grade or geology |
| **Channel samples** | Convert the sampling sheets of the workings (point + distance + width + *potencia* + grades) into drillhole data |
| **Narrow veins** | Hanging-wall and footwall surfaces, 3D prisms with grades kriged on the vein plane, validation, long section with edits, dilution and recovery, blocks between levels and raises, categories by rules you define |
| **Workings & sections** | Axes, closed solids, cuts and plans; sections also cut the **terrains** (Civil TIN surfaces): profile and contours |
| **Resources** | Composites, domains, variography, kriging, block models, grade-tonnage |
| **Geotechnics** | RMR / Q / GSI, Hoek-Brown, geotechnical block model, stereonets, wedges, stress, convergence, pillars |
| **Geological mapping** | Traces and projection along the dip |

## First steps

- **Try the narrow-vein example (10 minutes):** [examples/veta](examples/veta/README.md) goes from
  24 channel samples to tonnage by category, with the numbers you should see.
- **User guide:** [docs/GUIDE.md](docs/GUIDE.md) (English) · [docs/es/GUIA_USUARIO.md](docs/es/GUIA_USUARIO.md) (Spanish).
- **Cheat sheet of commands:** [docs/COMMANDS.md](docs/COMMANDS.md) · [docs/es/COMANDOS.md](docs/es/COMANDOS.md).

## Use it with an AI assistant (MCP)

Open CAD Studio exposes the drawing to AI clients with `OpenCADStudio --mcp`. Every Open Mining action is
a typed operation (`MIN_...` command with `key=value` arguments) that returns a machine-readable result
with diagnostics, so an assistant can load data, build the vein model, read the diagnostics and fix its
own inputs.

- **[docs/AI_QUICKSTART.md](docs/AI_QUICKSTART.md)** — start here: connecting, conventions, complete workflows, how to read diagnostics.
- **[AGENTS.md](AGENTS.md)** — rules for assistants: order of operations, what to verify, what never to invent.
- **[docs/MCP.md](docs/MCP.md)** — every operation: arguments, results, error codes, one worked example each (generated from the code). With Open CAD Studio v2026.47+ every operation is also a native AI tool with its JSON Schema.
- Ask the plugin itself: `MIN_HELP` lists the operations, `MIN_HELP op=vn_build` describes one.

## Documentation

| | English | Español |
|---|---|---|
| Install | [docs/INSTALL.md](docs/INSTALL.md) | [docs/es/INSTALAR.md](docs/es/INSTALAR.md) |
| User guide | [docs/GUIDE.md](docs/GUIDE.md) | [docs/es/GUIA_USUARIO.md](docs/es/GUIA_USUARIO.md) |
| Narrow veins | [docs/VEINS.md](docs/VEINS.md) | [docs/es/VETAS.md](docs/es/VETAS.md) |
| Commands | [docs/COMMANDS.md](docs/COMMANDS.md) | [docs/es/COMANDOS.md](docs/es/COMANDOS.md) |
| Geotechnics | [docs/GEOTECH.md](docs/GEOTECH.md) | |
| Resources | [docs/RESOURCES.md](docs/RESOURCES.md) | |

For AI assistants: [AI_QUICKSTART](docs/AI_QUICKSTART.md), [AGENTS.md](AGENTS.md), [MCP.md](docs/MCP.md).
Reference: [PLUGIN.md](docs/PLUGIN.md) (layers, XDATA, settings), [ARCHITECTURE.md](docs/ARCHITECTURE.md),
[MIGRATION.md](docs/MIGRATION.md), [TESTING.md](docs/TESTING.md), [CHANGELOG.md](CHANGELOG.md).
Examples: [veta](examples/veta/README.md), [terreno](examples/terreno/README.md), [canales](examples/canales/README.md), [geotechnics](examples/geotechnics/README.md).

## Develop

```bash
cargo test --workspace                       # all engines + plugin logic (mock CAD)
cargo clippy --workspace --all-targets -- -D warnings
cargo build -p opencad_mining --release      # target/release/libopencad_mining.{so,dll,dylib}
cargo run -p openmining_cli -- mcp-doc > docs/MCP.md   # regenerate the operations guide
```

Release: see [docs/RELEASING.md](docs/RELEASING.md) (version, CHANGELOG, tag `vX.Y.Z`; CI builds the
Linux / Windows / macOS assets).

## License

GPL-3.0-only (the host contract `ocs_plugin_api` is GPL-3.0-only).
