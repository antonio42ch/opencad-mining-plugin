# Open Mining

[English](README.md) · **Español**

Complemento de minería y geología para [Open CAD Studio](https://github.com/HakanSeven12/OpenCADStudio):
sondajes y canales de muestreo, labores subterráneas, mapeo geológico, estimación de recursos,
geotecnia y un **modelo de vetas angostas construido a partir de los datos** (superficies, prismas, leyes,
dilución, cubicación por bloques y categorías). Todo puede manejarlo también un **asistente de IA** por MCP.

Software libre, GPL-3.0. Versión **0.11.0** · API de plugins 28 · Windows, Linux y macOS (Apple Silicon).
Paneles y mensajes en español, inglés y portugués.

## Instalación (5 pasos, desde esta página)

1. Abra **Open CAD Studio** y vaya al **Administrador de plugins** (*Plugin Manager*).
2. Elija **Agregar repositorio** (*Add repository*) y pegue este enlace (o solo `antonio42ch/opencad-mining-plugin`):

   ```
   https://github.com/antonio42ch/opencad-mining-plugin
   ```
3. Elija la versión **v0.11.0** y pulse **Instalar** (*Install*).
4. **Reinicie** Open CAD Studio.
5. Aparece la pestaña **Minería** en la cinta. Para comprobar que funciona, escriba `MIN_STATUS` en la
   línea de comandos: informa qué hay cargado y qué hacer a continuación.

¿Problemas? Vea [docs/es/INSTALAR.md](docs/es/INSTALAR.md) (instalación manual, solución de problemas, actualizar).
Requiere Open CAD Studio **v2026.47 o superior** (API de plugins 28); para v2026.46 y anteriores use la versión v0.10.0.

## Qué puede hacer

| | |
|---|---|
| **Sondajes** | Cargar collar / survey / ensayos / geología (CSV, XLSX), validar, desurvey, dibujar en 3D por ley o geología |
| **Canales de muestreo** | Convertir las hojas de muestreo de las labores (punto + distancia + ancho + potencia + leyes) en datos tipo sondaje |
| **Vetas angostas** | Superficies de techo y piso, prismas 3D con leyes por kriging en el plano de la veta, validación, longitudinal con ediciones, dilución y recuperación, bloques entre niveles y chimeneas, categorías con las reglas que usted defina |
| **Labores y secciones** | Ejes, sólidos cerrados, cortes y plantas; las secciones también cortan los **terrenos** (superficies TIN de Civil): perfil y curvas de nivel |
| **Recursos** | Compósitos, dominios, variografía, kriging, modelo de bloques, curva ley-tonelaje |
| **Geotecnia** | RMR / Q / GSI, Hoek-Brown, modelo geotécnico de bloques, estereogramas, cuñas, esfuerzos, convergencia, pilares |
| **Mapeo geológico** | Trazas y proyección según el buzamiento |

## Primeros pasos

- **Pruebe el ejemplo de veta (10 minutos):** [examples/veta](examples/veta/README.md) va de 24 muestras de
  canal al tonelaje por categoría, con las cifras que debe ver.
- **Guía de usuario:** [docs/es/GUIA_USUARIO.md](docs/es/GUIA_USUARIO.md).
- **Hoja rápida de comandos:** [docs/es/COMANDOS.md](docs/es/COMANDOS.md).

## Úselo con un asistente de IA (MCP)

Open CAD Studio expone el dibujo a clientes de IA con `OpenCADStudio --mcp`. Cada acción de Open Mining es una
operación tipada (comando `MIN_...` con argumentos `clave=valor`) que devuelve un resultado legible por
máquina con diagnósticos, de modo que un asistente puede cargar datos, construir el modelo de veta, leer los
diagnósticos y corregir sus propios datos de entrada.

- **[docs/AI_QUICKSTART.md](docs/AI_QUICKSTART.md)** — empiece aquí (en inglés): conexión, convenciones, flujos completos, cómo leer los diagnósticos.
- **[AGENTS.md](AGENTS.md)** — reglas para asistentes: orden de las operaciones, qué verificar, qué no inventar nunca.
- **[docs/MCP.md](docs/MCP.md)** — todas las operaciones: argumentos, resultados, códigos de error y un ejemplo de cada una (generado del código). Con Open CAD Studio v2026.47+ cada operación es además una herramienta de IA nativa con su esquema JSON.
- Pregúntele al propio plugin: `MIN_HELP` lista las operaciones y `MIN_HELP op=vn_build` describe una.

Sugerencias de lo que puede pedirle a su asistente: [docs/es/GUIA_USUARIO.md](docs/es/GUIA_USUARIO.md#pedírselo-a-un-asistente-de-ia).

## Documentación

| | Español | English |
|---|---|---|
| Instalar | [docs/es/INSTALAR.md](docs/es/INSTALAR.md) | [docs/INSTALL.md](docs/INSTALL.md) |
| Guía de usuario | [docs/es/GUIA_USUARIO.md](docs/es/GUIA_USUARIO.md) | [docs/GUIDE.md](docs/GUIDE.md) |
| Vetas angostas | [docs/es/VETAS.md](docs/es/VETAS.md) | [docs/VEINS.md](docs/VEINS.md) |
| Comandos | [docs/es/COMANDOS.md](docs/es/COMANDOS.md) | [docs/COMMANDS.md](docs/COMMANDS.md) |
| Geotecnia | | [docs/GEOTECH.md](docs/GEOTECH.md) |
| Recursos | | [docs/RESOURCES.md](docs/RESOURCES.md) |

Ejemplos con datos: [veta](examples/veta/README.md), [terreno](examples/terreno/README.md), [canales](examples/canales/README.md),
[geotecnia](examples/geotechnics/README.md). Cambios de cada versión: [CHANGELOG.md](CHANGELOG.md).

## Licencia

GPL-3.0-only (el contrato con el host `ocs_plugin_api` es GPL-3.0-only).
