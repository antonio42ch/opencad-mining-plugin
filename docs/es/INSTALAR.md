# Instalar Open Mining

[English](../INSTALL.md)

## Requisitos

- **Open CAD Studio** con API de plugins 23 (probado con v2026.46).
- Windows (x86_64), Linux (x86_64) o macOS con Apple Silicon (aarch64). No hay versión para macOS Intel.
- Nada más: el plugin es un solo archivo, sin bibliotecas adicionales ni licencias.

## Instalar con el enlace del repositorio (recomendado)

1. Abra Open CAD Studio y vaya al **Administrador de plugins** (*Plugin Manager*).
2. **Agregar repositorio** (*Add repository*) y pegue:

   ```
   https://github.com/antonio42ch/opencad-mining-plugin
   ```

   (también sirve la forma corta `antonio42ch/opencad-mining-plugin`).
3. Elija la versión **v0.10.0** y pulse **Instalar** (*Install*). El administrador descarga el archivo de su
   sistema operativo.
4. **Reinicie** Open CAD Studio.
5. Compruebe: la cinta tiene la pestaña **Minería**, el administrador lista *Open Mining 0.10.0* como
   habilitado y al escribir `MIN_STATUS` en la línea de comandos aparece lo que hay cargado.

El plugin guarda su configuración **dentro de cada dibujo**: un dibujo abierto en otro equipo con el plugin
instalado muestra los mismos paneles y valores. Los archivos que escribe (canales convertidos, compósitos,
prismas, bloques…) quedan **junto al dibujo**, con nombre `<dibujo>_…`; guarde el dibujo antes.

## Actualizar

En el Administrador de plugins elija la versión nueva de *Open Mining*, instálela y reinicie Open CAD
Studio. Sus dibujos conservan la configuración. El [CHANGELOG](../../CHANGELOG.md) (en inglés) lista lo que
agrega cada versión.

## Instalación manual

Úsela si el Administrador de plugins no llega a GitHub.

1. En la página **Releases** del repositorio abra v0.10.0 y descargue los dos archivos de su sistema:

   | Sistema | Archivo del plugin | Además |
   |---|---|---|
   | Windows | `opencad.mining-windows-x86_64.dll` | `plugin.toml` |
   | Linux | `opencad.mining-linux-x86_64.so` | `plugin.toml` |
   | macOS (Apple Silicon) | `opencad.mining-macos-aarch64.dylib` | `plugin.toml` |

2. Copie ambos archivos a la **carpeta de plugins del usuario** de Open CAD Studio (su ubicación está en la
   documentación del programa) y reinícielo.

## Idioma

La interfaz sigue el idioma del sistema (español, inglés o portugués). Para forzar uno, defina la variable de
entorno `OPENMINING_LANG` como `es`, `en` o `pt` antes de abrir Open CAD Studio.

## Solución de problemas

| Síntoma | Qué revisar |
|---|---|
| No aparece la pestaña *Minería* tras reiniciar | El administrador debe listar *Open Mining* como habilitado; si muestra un error, suele ser un programa anterior a la API 23: actualice Open CAD Studio. |
| Falta solo en macOS | Solo hay versión para Apple Silicon. |
| `MIN_STATUS` dice *comando desconocido* | El plugin no está cargado: vea las dos filas anteriores. |
| Un comando de carga dice que no puede leer el archivo | Ponga entre comillas las rutas con espacios (`"C:\Mis datos\collar.csv"`); el CSV puede usar `;` o `,` y coma decimal: se reconocen ambos. |
| Los archivos no aparecen junto al dibujo | Guarde primero el dibujo: el plugin necesita su ruta. |
| Un panel sale vacío o en otro idioma | Cierre y abra el panel; revise `OPENMINING_LANG`. |

El programa escribe los errores de carga de plugins en la línea de comandos y en `automation/gui.log` de su
carpeta de configuración. Si el problema sigue, abra una incidencia (*issue*) en el repositorio con la
versión (`MIN_STATUS`), su sistema y ese registro.

## Desinstalar

Quite *Open Mining* en el Administrador de plugins (o borre los dos archivos copiados) y reinicie. Los
dibujos conservan sus entidades (son polilíneas 3D, mallas y textos normales) y solo dejan de tener los
paneles de Open Mining.
