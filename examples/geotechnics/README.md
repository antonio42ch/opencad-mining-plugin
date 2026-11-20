# Ejemplo para probar Geotecnia

Un depósito ficticio: 4 sondajes verticales de 160 m alrededor de una galería recta de 100 m.
Los archivos usan `;` y coma decimal (como Excel en español) salvo `survey`, `assays`, `lab` y
`mapping_*`, que usan coma como separador: el plugin reconoce ambos.

| Archivo | Contenido |
|---|---|
| `collar.csv`, `survey.csv`, `assays.csv` | 4 sondajes GT-01…GT-04 (collar en cota 3100) con Cu % |
| `logging.csv` | logueo geotécnico: 33 intervalos de 20 m (Andesita 0–80, Brecha 80–120, Granodiorita 120–160) |
| `lab.csv` | UCS, mi, Ei, Poisson y densidad de las 3 litologías |
| `mapping_chainage.csv` | 5 tramos de la galería por **progresiva** (PK 10, 30, 50, 70, 90) |
| `mapping_coords.csv` | 3 puntos por **coordenadas** con RMR y Q ya calculados |
| `joints.csv` | 3 discontinuidades (se cargan; se analizan en la fase de estructuras) |

Casos intencionales dentro de `logging.csv`:
- **GT-01 60–80**: sin RQD ni espaciado → queda **sin clasificar** (gris).
- **GT-02 40–60**: sin RQD ni espaciado, pero con 3,5 fracturas/m → se **derivan** (RMR 77).
- **GT-02 80–100**: Jn = 30, fuera de la tabla de Q → RMR 45 pero **sin Q** y reportado como inválido.
- **GT-99**: sondaje que no existe → **no se dibuja** (reporte: «sin ubicar»).
- La UCS se toma del **laboratorio** según la litología.

## Pasos

Ubica la carpeta: en WSL `~/pluging_ocad/opencad-mining/examples/geotechnics`; desde Windows
`\\wsl$\<tu distribución>\home\usuario2\pluging_ocad\opencad-mining\examples\geotechnics`.

**1. Sondajes.** Documento nuevo → pestaña **Minería** → **Sondajes → Panel**. Elige
`collar.csv`, `survey.csv` y `assays.csv` → **Cargar** → **Dibujar**. Deben verse 4 trazas verticales.

**2. Eje de la galería.** **Ejes → Panel**: E `500000`, N `8500000`, Z `3000`, rumbo `N90E`
(o `90`), largo `100`, pendiente `0` → **Recta**. Aparece una polilínea de 100 m al este.

**3. Cargar datos geotécnicos.** **Geotecnia → Datos**: logueo `logging.csv`, mapeo
`mapping_chainage.csv`, laboratorio `lab.csv`, discontinuidades `joints.csv` → **Cargar**.
Esperado en la línea de comandos: logueo 33 registros, mapeo 5, laboratorio 3 litologías,
3 discontinuidades. No debe pedir columnas (todas se reconocen).

**4. Clasificar y dibujar.** Selecciona el eje (o deja que lo pida) → **Clasificar y dibujar**.
Esperado:
- `Clasificación: 38 registros · RMR 37 · Q 36 · GSI 37 · sin clasificar 1`
- `Clases RMR: I 11 · II 16 · III 9 · IV 1 · V 0`
- datos faltantes: `RQD 1`, `espaciado 1`; un valor fuera de tablas (Jn = 30, fila de GT-02)
- `1 registros sin ubicar` (GT-99)
- 42 entidades dibujadas (32 intervalos en `MIN_GT_SONDAJE`; 5 puntos y 5 etiquetas en `MIN_GT_MAPEO`): la Granodiorita y la Andesita en azul/verde,
  la Brecha en amarillo; el intervalo sin clasificar en gris; en la galería, puntos con etiquetas
  `RMR · Q · GSI` (PK 10: RMR 84, Q 20; PK 70: RMR 34, Q 0,8, naranja).
- Cambia **Colorear por** a *Clase Q* o *GSI* y vuelve a clasificar para ver otros colores.

**5. Probar «no se inventa».** Activa **Completar RMR o Q con la correlación** → **Clasificar**:
el registro con Jn fuera de tabla recibe Q desde su RMR y el informe lo cuenta en
*Valores no medidos directamente*. Desactívalo después.

**6. Hoek-Brown.** Panel **Hoek-Brown**: litología `Andesita` → **Usar datos clasificados**.
Esperado: GSI 74 (mediana de 15 registros), σci 140, mi 25 y
`mb ≈ 9,88 · s ≈ 0,0556 · a ≈ 0,501 · σc del macizo ≈ 33 MPa`, con el gráfico de la envolvente.
Prueba también valores a mano: GSI 50, mi 10, σci 100, D 0 → `mb 1,677 · s 0,00387 · a 0,506`.

**7. Modelo de bloques.** Panel **Modelo** → **Estimar geotecnia**. Crea un modelo con bloques
de 10 × 10 × 12 m cubriendo los registros y reporta `Modelo geotécnico: N de M bloques estimados
(gt_rmr, gt_gsi, gt_logq, gt_class)`. Luego **Ver modelo** con *Clase de macizo*: los bloques de
Brecha (profundos, 80–120 m bajo el collar) salen en clase III (amarillo) y el resto en I–II.
Prueba la vista de niveles, secciones y cubos desde el panel *Modelo* de Recursos.

**8. Mapeo por coordenadas.** En **Datos** cambia el mapeo a `mapping_coords.csv` → **Cargar**
→ **Clasificar**: no pide eje; 3 puntos con clases II, III y IV (usa los RMR y Q ya dados:
*RMR ingresado*, *Q ingresado* en el informe).

**9. Persistencia.** Guarda el DWG, ciérralo y ábrelo: el plugin restaura la configuración y,
al clasificar o estimar de nuevo, recarga las tablas de las rutas guardadas (mensaje
*Datos geotécnicos recargados*). Si moviste los archivos, pedirá cargarlos otra vez.

## Por línea de comandos (sin interfaz)

```
MIN_DH_LOAD "<carpeta>/collar.csv" "<carpeta>/survey.csv" "<carpeta>/assays.csv"
MIN_GT_LOAD "<carpeta>/logging.csv" - "<carpeta>/lab.csv"
MIN_GT_CLASSIFY
MIN_GT_HB
MIN_GT_ESTIMATE
MIN_GT_VIEW
```
(`-` conserva el valor del panel; sin mapeo por progresiva no pide el eje.)
