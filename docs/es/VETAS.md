# Vetas angostas

[English](../VEINS.md) · [Ejemplo con cifras](../../examples/veta/README.md) · [Operaciones para IA](../MCP.md)

Las vetas angostas no se modelan con el modelo de bloques regular. El modelo de veta se construye a partir de los
datos en etapas; cada una es una operación que un asistente de IA puede ejecutar, comprobar y ajustar (vea
[MCP.md](../MCP.md)):

| Etapa | Estado | Qué hace |
|---|---|---|
| A — canales | hecha | Canales de muestreo de las labores convertidos en datos tipo sondaje (`MIN_CH_CONVERT`, vea [examples/canales](../../examples/canales/README.md)) |
| B1 — superficies | hecha | Superficies de techo y piso desde contactos mapeados e intersecciones de veta (`MIN_VN_SURFACES`) |
| B2 — prismas y leyes | hecha | Prismas 3D entre las superficies, leyes por acumulación en el plano de la veta, variografía en el plano, validación contra canales, polígono límite, distancia máxima, ediciones en la longitudinal (`MIN_VN_VARIOGRAM`, `MIN_VN_ESTIMATE`, `MIN_VN_LONG`, `MIN_VN_EDIT`) |
| B3 — cubicación | hecha | Dilución y recuperación, bloques entre niveles y chimeneas con pilares, lados muestreados, categorías por una tabla de reglas ordenada que se maneja desde MCP (`MIN_VN_CUBICATE`, `MIN_VN_RULES`, `MIN_VN_BUILD`) |
| B4 — ramales | prevista | Varias vetas, solapes informados, recálculo incremental al agregar canales |

**Orden de trabajo:** canales (o sondajes) cargados → superficies → variograma → prismas → (longitudinal,
ediciones) → cubicación. `MIN_VN_BUILD` ejecuta todo lo posterior a la carga de datos en una sola llamada; cada
etapa también se puede repetir sola tras cambiar un parámetro. Todas leen los parámetros guardados de la veta
(los valores del panel): lo que fije en un panel, en un comando o con un asistente es lo mismo.

**Capas y archivos**

| Capa | Contenido | Color |
|---|---|---|
| `VETA_TECHO`, `VETA_PISO` | *sus* polilíneas de contacto mapeadas (los nombres son parámetros) | — |
| `NIVELES`, `CHIMENEAS` | *sus* polilíneas de niveles y chimeneas (parámetros) | — |
| `MIN_VN_TECHO` / `MIN_VN_PISO` | superficies de techo / piso | 30 / 4 |
| `MIN_VN_PRISMAS` | prismas | según atributo |
| `MIN_VN_LONG` | longitudinal, muestras, polígonos de edición | 7 |
| `MIN_VN_BLOQUES` | labores y rótulos de bloques en la longitudinal | 1 |

Archivos junto al dibujo: `<dibujo>_veta_<nombre>.json` (superficies), `_prismas.csv`, `_bloques.csv`.

## B1 — Superficies de techo y piso

Panel **Veta: superficies** (grupo de la cinta *Vetas*), comando `MIN_VN_SURFACES`.

**Contactos**

- Contactos mapeados: polilíneas (3D, polilíneas, líneas) en las capas de techo y piso (por defecto
  `VETA_TECHO` / `VETA_PISO`, se admiten comodines) y/o elegidas como *handles* `hw=` / `fw=`. Se toman puntos en
  los vértices y al menos cada *densificar* metros. Son entidades del dibujo: se suma de vuelta el origen local
  del dibujo para trabajar en coordenadas reales.
- Intersecciones de veta: los intervalos de geología de los sondajes y canales cargados cuyo atributo
  (`vein_attr`, por defecto `tipo`) tiene uno de los valores de veta (`vein_values`, por defecto `VETA`),
  unidos por sondaje. Cada intersección da un punto de techo y uno de piso (el extremo más alejado según la
  normal de la veta es el techo).

**Marco de la veta.** Un plano de referencia ajustado por mínimos cuadrados a todos los contactos (o un
buzamiento / dirección de buzamiento escritos) define coordenadas locales: `u` según el rumbo (horizontal), `v`
según el buzamiento hacia arriba y `w` según la normal hacia el techo. Las vetas verticales y muy inclinadas se
tratan igual que las tendidas.

**Superficies.** El desplazamiento normal `w` de cada caja se interpola en una malla común de `step` metros sobre
los contactos más un margen, por kriging ordinario (modelo esférico, meseta = varianza de los desplazamientos,
alcance = el radio de búsqueda salvo que se indique) o inverso de la distancia. Los nodos con menos de
`min_contacts` contactos dentro del radio quedan vacíos: el radio es la extrapolación máxima. Ambas cajas se
dibujan como mallas en `MIN_VN_TECHO` y `MIN_VN_PISO` (XDATA `VN_SURFACE`: `vein`, `side`) y se guardan en
`<dibujo>_veta_<nombre>.json` para el modelo de prismas.

**Diagnósticos** (en la línea de comandos, el panel y el JSON del resultado):

- ajuste a los contactos: RMS por caja y los diez contactos más alejados de su superficie (polilínea o sondaje
  de origen, residuo en metros);
- cruces: nodos donde el piso queda sobre el techo (contacto malo, caja equivocada o un acuñamiento que se
  acota con un polígono en B2);
- nodos con una sola caja (amplíe el radio o agregue contactos);
- rango de potencia entre las cajas y nodos más delgados que *potencia mínima esperada*.

Un asistente los usa para iterar: por ejemplo, un residuo grande en un contacto → revisar o quitar esa polilínea;
cruces cerca de un sondaje → su intervalo de veta puede estar mal logueado; muchos nodos con una sola caja →
radio mayor o más mapeo.

## B2 — Modelo de prismas y leyes

Panel **Veta: modelo de prismas** (grupo *Vetas* → *Modelo*).

**Compósitos.** Cada intersección de veta de los sondajes y canales cargados (B1) es una muestra sobre el plano de
la veta: su ley (`grade`, p. ej. `au`) es la media ponderada por longitud de los ensayos dentro de la
intersección; su potencia real es la media ponderada por longitud de la columna de potencia (`thickness_col`,
en canales: `potencia`), o el largo de la intersección según la normal de la veta si falta esa columna;
acumulación = ley × potencia.

**Variografía en el plano** (`MIN_VN_VARIOGRAM`). Variogramas de la acumulación sobre el plano de la veta:
omnidireccional, según el rumbo y según el buzamiento (±22,5°), con un modelo ajustado (esférico, exponencial o
gaussiano) cuya pepita, meseta y alcances en rumbo y buzamiento se escriben en los parámetros (edítelos antes de
estimar si hace falta). El gráfico se muestra en el panel.

**Estimación** (`MIN_VN_ESTIMATE`). Una malla de prismas de `block_u` × `block_v` metros sobre el plano de la veta
cubre las superficies. En cada prisma, la acumulación y la potencia se estiman por kriging de bloques (ordinario,
discretización `disc` × `disc`) con las mismas muestras y pesos, y ley = acumulación / potencia. El volumen del
prisma sale de las dos cajas en sus esquinas (B1); toneladas = volumen × densidad. Se dejan fuera los prismas
que están fuera del polígono límite, a más de `max_distance` de todo compósito, donde falta una caja o el piso
queda sobre el techo, dentro de una edición *excluir*, o sin `min_samples` compósitos en el radio de búsqueda;
se informa cada cuenta.

Resultados: prismas dibujados como mallas cerradas en `MIN_VN_PRISMAS` (XDATA `VN_PRISM`: `vein`, `i`, `j`, ley,
`thickness`, `tonnes`) coloreados por ley, potencia, acumulación, distancia a la muestra más cercana o varianza
de kriging; un CSV `<dibujo>_veta_<nombre>_prismas.csv` (`i, j, u, v, x, y, z, thickness, area, volume, tonnes,
<ley>, accumulation, distance, samples, variance, edited`); y los totales (área, volumen, toneladas, ley y
potencia medias).

**Validación contra las muestras**: validación cruzada de la acumulación ocultando el propio sondaje de cada
muestra (sesgo y correlación), y cada compósito contra el prisma que lo contiene (medias, correlación y las
mayores diferencias, por sondaje).

**Longitudinal y ediciones** (`MIN_VN_LONG`, `MIN_VN_EDIT`). La longitudinal se dibuja plana en el dibujo (el rumbo
hacia la derecha, el buzamiento hacia arriba) en un punto base (`at=`, por defecto junto al modelo y luego el
último): prismas como caras coloreadas, compósitos con rótulos `sondaje ley/potencia`, el límite (amarillo) y los
polígonos de edición (magenta), en `MIN_VN_LONG`. Dibuje una polilínea cerrada sobre ella y ejecute
`MIN_VN_EDIT` con una acción:

| acción | efecto |
|---|---|
| `exclude` | los prismas dentro se dejan fuera (estéril, ya minado, pilar) |
| `grade` | los prismas dentro toman `value` como ley |
| `thickness` | los prismas dentro toman `value` como potencia (centrada en la veta) |
| `boundary` | el polígono pasa a ser el límite del modelo |
| `undo` / `clear` / `list` | quita la última edición / todas las ediciones y el límite / las lista |

Las ediciones se guardan por veta en los parámetros del proyecto, se aplican en orden en cada estimación, y el
modelo y la longitudinal se actualizan tras cada edición.

## B3 — Cubicación, bloques y categorías

Panel **Veta: cubicación** (grupo *Vetas* → *Cubicación*), comando `MIN_VN_CUBICATE`, tras `MIN_VN_ESTIMATE`.

**Dilución y recuperación** (por prisma). Un prisma más delgado que el ancho mínimo de minado (`mmw`) se ensancha
hasta él, mitad en cada caja; se suma encima la sobre-excavación de techo y piso (`overbreak_hw`,
`overbreak_fw`). La roca agregada tiene su propia densidad (`density_waste`) y ley en cada caja (`grade_hw`,
`grade_fw`):

```
toneladas diluidas = área × (potencia × densidad + (hw + fw) × densidad de la caja)
ley diluida        = (t de mena × ley + t hw × grade_hw + t fw × grade_fw) / toneladas diluidas
recuperado         = toneladas diluidas × recuperación
```

**Bloques entre labores.** Los niveles (galerías) y chimeneas son polilíneas en sus capas (por defecto `NIVELES`
/ `CHIMENEAS`) o elegidas (`levels=`, `raises=`); se proyectan sobre el plano de la veta. Los niveles se numeran
de abajo hacia arriba `L1, L2, …` y las chimeneas de izquierda a derecha `R1, R2, …` según el rumbo; cada prisma
pertenece al panel entre ellos, llamado `L1-L2/R1-R2` (`<L1`, `>L2`, `<R1`, `>R1` fuera de la primera o última
labor). Los prismas dentro del puente sobre cada nivel (`level_pillar` m) o del pilar centrado en cada chimenea
(`raise_pillar` m de ancho) se separan y se informan como pilares.

**Lados muestreados.** Cada lado de un bloque (nivel inferior, nivel superior, chimenea izquierda, chimenea
derecha) está muestreado cuando al menos `side_min` compósitos están a menos de `side_tolerance` metros de esa
labor dentro del tramo del bloque. Un lado sin labor (borde abierto del modelo) no cuenta. El resultado lista
los lados como `true` / `false` / `null`.

**Categorías por reglas** (se manejan desde MCP). La tabla `rules` es una lista ordenada de `[categoría, lados
muestreados mínimos, distancia media máxima, muestras mínimas, ley diluida mínima]`; una celda vacía no se
revisa. Cada bloque toma la **primera** regla que cumple; los que no cumplen ninguna quedan sin categoría. El
plugin trae reglas de ejemplo (Medido 3 lados ≤ 15 m, Indicado 2 lados ≤ 30 m, Inferido 1 lado): reemplácelas por
las normas de su empresa, desde el panel o con `rules=[[...], ...]`. `MIN_VN_RULES sides=2 dist=20 samples=4
grade=3.5` indica qué regla cumple un bloque con esos valores y por qué fallan todas las anteriores, de modo que
las normas se prueban antes de cubicar. Cada bloque del resultado trae además su `why`.

**Salidas.** Línea de comandos y panel: una línea por bloque (categoría, toneladas y leyes in situ, diluidas y
recuperadas, lados muestreados ■ □ ·, distancia media) y totales por categoría; el JSON del resultado
(`blocks`, `totals`, `pillars`); `<dibujo>_veta_<nombre>_bloques.csv`; y en la longitudinal las labores (rojo) y
los rótulos de bloque coloreados por categoría (`MIN_VN_BLOQUES`, XDATA `VN_BLOCK`). Con *Colorear por* =
*Categoría* la longitudinal también se colorea por categoría.

**Construir todo** (`MIN_VN_BUILD`): ejecuta `vn_surfaces`, `vn_variogram` (si `fit_variogram`), `vn_estimate` y
`vn_cubicate` (si `tonnage`), se detiene en el primer paso que falla y lo nombra. Acepta todos los argumentos de
esos pasos (`step=`, `radius=`, `grade=`, `block_u=`, `max_distance=`, `mmw=`, `levels=`, `rules=`…); los que
usted da se guardan y los demás conservan su valor actual.

## Cómo leer los diagnósticos

| Usted ve | Significa | Qué hacer |
|---|---|---|
| `crossings > 0` (superficies) | el piso queda sobre el techo en algunos nodos | mire `worst`: un contacto en la caja equivocada o un intervalo `tipo` mal logueado; corríjalo o quítelo |
| `one_side > 0` | una caja está definida donde la otra no | suba `radius`, o mapee / muestree más ahí |
| `rms_hw` / `rms_fw` altos, `worst[0].residual` grande | la superficie no sigue algún contacto | revise ese contacto (nombre de origen); pruebe el otro `method` o un `radius` menor |
| `thin > 0` | nodos más delgados que `min_thickness` | acuñamiento: acótelo con un polígono, o acéptelo |
| `from_intercepts = 0` | no se encontraron los intervalos de veta | revise `vein_attr` / `vein_values` contra la columna de geología, y que los canales / sondajes estén cargados |
| `composites < 3` | pocas intersecciones con ley | cargue más datos; `missing_grade` dice cuántas no tienen ensayo |
| `cv_correlation` cerca de 0 y pepita alta | las leyes tienen poca continuidad espacial a este espaciamiento | trate con cautela las leyes locales y mire el variograma (pepita, alcance) |
| `mean_composite` ≠ `mean_prism` en más de 10 % | la estimación tiene sesgo respecto de las muestras | revise los alcances del variograma, `max_distance`, muestras agrupadas; `worst` nombra los sondajes |
| `out_distance`, `out_walls` grandes | se descartaron muchas celdas | suba `max_distance` o `radius`, o acepte el límite |
| un bloque no está en la categoría esperada | falló una regla | lea el `why` del bloque; pruebe con `MIN_VN_RULES` |

## Límites

- Una veta a la vez (`vein=` la nombra; en un mismo dibujo se pueden guardar varias, una tras otra). Los ramales y la
  revisión de solapes entre vetas están previstos (B4).
- Los contactos y las labores deben existir como polilíneas (o venir de las intersecciones); el plugin no traza los
  contactos por sí mismo.
- Las leyes se interpolan, no se simulan; no hay clasificación de recursos por varianza de kriging: las categorías
  salen de su tabla de reglas.
- Las reglas que trae la tabla son ejemplos, no una norma.
