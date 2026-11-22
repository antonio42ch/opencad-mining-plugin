# Installing Open Mining

[Español](es/INSTALAR.md)

## Requirements

- **Open CAD Studio** with plugin API 23 (tested with v2026.46).
- Windows (x86_64), Linux (x86_64) or macOS on Apple Silicon (aarch64). There is no Intel macOS build.
- Nothing else: the plugin is a single file and needs no extra libraries or licences.

## Install from the repository link (recommended)

1. Open Open CAD Studio and go to the **Plugin Manager**.
2. **Add repository** and paste:

   ```
   https://github.com/antonio42ch/opencad-mining-plugin
   ```

   (the short form `antonio42ch/opencad-mining-plugin` also works).
3. Pick the release **v0.10.0** and press **Install**. The Plugin Manager downloads the file for your
   operating system.
4. **Restart** Open CAD Studio.
5. Check it: the ribbon has a **Mining** tab (*Minería* with the Spanish interface), the Plugin Manager lists
   *Open Mining 0.10.0* as enabled, and typing `MIN_STATUS` in the command line prints what is loaded.

The plugin keeps its settings **inside each drawing**, so a drawing opened on another computer with the
plugin installed shows the same panels and values. Files the plugin writes (converted channels, composites,
prisms, blocks…) go **next to the drawing**, named `<drawing>_…`; save the drawing first.

## Update

Open the Plugin Manager, choose the newer release of *Open Mining* and install it; restart Open CAD Studio.
Your drawings keep their settings. The [CHANGELOG](../CHANGELOG.md) lists what each version adds.

## Manual installation

Use it when the Plugin Manager cannot reach GitHub.

1. On the repository's **Releases** page open v0.10.0 and download the two files for your system:

   | System | Plugin file | Also |
   |---|---|---|
   | Windows | `opencad.mining-windows-x86_64.dll` | `plugin.toml` |
   | Linux | `opencad.mining-linux-x86_64.so` | `plugin.toml` |
   | macOS (Apple Silicon) | `opencad.mining-macos-aarch64.dylib` | `plugin.toml` |

2. Copy both files into Open CAD Studio's **user plugins folder** (see the host's documentation for its
   location on your system) and restart it.

## Language

The interface follows the system language (Spanish, English or Portuguese). To force one, set the
environment variable `OPENMINING_LANG` to `es`, `en` or `pt` before starting Open CAD Studio.

## Troubleshooting

| Symptom | What to check |
|---|---|
| No *Mining* tab after restarting | The Plugin Manager must list *Open Mining* as enabled; if it shows an error, it is usually a host older than plugin API 23: update Open CAD Studio. |
| The tab is missing only on macOS | Only Apple Silicon has a build. |
| `MIN_STATUS` says *unknown command* | The plugin is not loaded: see the two rows above. |
| A load command says the file cannot be read | Quote paths that contain spaces (`"C:\My data\collar.csv"`); CSV may use `;` or `,` and a decimal comma — both are recognised. |
| Files do not appear next to the drawing | Save the drawing first; the plugin needs its path. |
| A panel looks empty or in another language | Close and reopen the panel; check `OPENMINING_LANG`. |

The host writes plugin load errors to the command line and to `automation/gui.log` in its configuration folder.
If the problem remains, open an issue on the repository with the version (`MIN_STATUS`), your system and
that log.

## Uninstall

Remove *Open Mining* in the Plugin Manager (or delete the two files you copied) and restart. Drawings keep
their entities (they are ordinary 3D polylines, meshes and text) and just stop having the Open Mining panels.
