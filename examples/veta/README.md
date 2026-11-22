# Ejemplo: de los canales de muestreo al tonelaje de una veta

Con los datos de [`../canales`](../canales/README.md) se construye todo el modelo de la veta:
superficies de techo y piso, prismas con leyes, bloques entre dos niveles y una chimenea, y
categorías. No hace falta ningún dibujo previo: los contactos salen de los propios canales.

**La veta del ejemplo:** rumbo N30E, buzando unos 66° hacia el NO (dirección de buzamiento ≈ 300°),
potencia entre 0,2 y 1,3 m, muestreada en dos galerías (niveles 3950 y 4000, 50 m de diferencia).
Los datos son **sintéticos**: sirven para aprender el flujo, no para sacar conclusiones geológicas.

## Antes de empezar

1. Documento nuevo en OpenCAD Studio, pestaña **Minería**.
2. Descargue la carpeta `examples/canales` del repositorio (o clone el repositorio) y anote su ruta.

## Paso 1 — Canales como sondajes

**Minería → Sondajes → Canales** (panel *Canales de muestreo*): elija
`puntos_topograficos.csv` y `muestreo_canales.csv` → **Convertir canales**.

Esperado: `Canales: 24 convertidos (72 muestras)`.

```
MIN_CH_CONVERT points="<carpeta>/puntos_topograficos.csv" sheet="<carpeta>/muestreo_canales.csv"
```

## Paso 2 — Niveles y chimenea (para el tonelaje)

La cubicación necesita las labores como polilíneas 3D. Dibuje con **Polilínea 3D** (`3DPOLY`) estas
tres, con las coordenadas de los puntos topográficos, y póngalas en las capas indicadas
(si el dibujo usa un origen local, reste el origen):

| Capa | Polilínea (E, N, Cota) |
|---|---|
| `NIVELES` | `500000,8500000,3950` → `500075,8500129.9,3951.2` (nivel 3950, PT-101 a PT-104) |
| `NIVELES` | `500020,8499990,4000` → `500070,8500076.6,4000.6` (nivel 4000, PT-201 a PT-203) |
| `CHIMENEAS` | `500050,8500086.6,3950.8` → `500070,8500076.6,4000.6` (chimenea, PT-103 a PT-203) |

Con un asistente (MCP) no hace falta ponerlas en capas: se pasan sus *handles* (`levels=`, `raises=`).

## Paso 3 — Todo el modelo con un clic

**Minería → Vetas → Cubicación**, sección **Construir todo** (con *Ajustar el variograma* y *Cubicar* activos) →
botón **Construir todo**. Los demás valores se dejan por defecto. O por comando:

```
MIN_VN_BUILD vein=EJ1 vein_attr=tipo vein_values=VETA grade=au thickness_col=potencia
```

El resultado aparece en el panel y en la línea de comandos, y se dibuja en 3D (capas `MIN_VN_TECHO`,
`MIN_VN_PISO`, `MIN_VN_PRISMAS`).

## Qué debe ver

**Superficies.** Plano ajustado **66° / 300°**; 24 + 24 puntos de contacto, todos de
las intersecciones de los canales (la veta se reconoce porque `tipo = VETA`). Potencia entre
0,21 y 1,34 m (media 0,72 m); ajuste a los contactos 0,03–0,04 m; **0 cruces** (el piso nunca queda sobre
el techo); 18 nodos con una sola superficie.

**Compósitos y variograma.** 24 compósitos (uno por canal): ley media ponderada **5,87 g/t**, potencia
media 0,77 m. El variograma de la acumulación (ley × potencia) se ajusta con un **efecto pepita alto**
(13,0 de 34,2 de meseta total) y alcance de 108 m: los canales de este ejemplo tienen poca
continuidad espacial. Es lo esperado en datos sintéticos aleatorios; con datos reales mire este
variograma antes de fiarse de la estimación.

**Prismas.** 321 prismas de 5 × 5 m (área 8 025 m², volumen 5 912 m³, **15 963 t** con densidad 2,7),
ley media **6,82 g/t**, potencia media 0,74 m. Quedan fuera 69 celdas por la distancia máxima (40 m) y
30 por no tener las dos superficies.

**Validación.** La validación cruzada de la acumulación da correlación **0,05**: casi nula, coherente con
el efecto pepita alto. Comparados con su prisma, los compósitos tienen ley media 5,58 frente a 6,10 y
correlación 0,60; el compósito `CH-4000-201-03` (18,99 g/t) es el que más difiere del prisma (7,69).

**Cubicación.** Con las reglas de ejemplo y los valores de dilución por defecto
(ancho mínimo de minado 0,8 m, sobre-excavación 0,10 / 0,05 m, recuperación 90 %):

| Bloque | Categoría | Lados muestreados | t in situ | Ley | t diluidas | Ley diluida |
|---|---|---|---|---|---|---|
| `L1-L2/>R1` (principal) | Indicado | 3 | 11 801 | 6,89 | 15 119 | 5,38 |
| `L1-L2/<R1` | Indicado | 2 | 1 794 | 6,39 | 4 018 | 2,85 |
| `<L1/>R1` | Inferido | 1 | 1 059 | 5,32 | 1 490 | 3,78 |
| `>L2/>R1` | Inferido | 1 | 1 071 | 8,36 | 1 238 | 7,23 |
| `<L1/<R1` | Inferido | 1 | 238 | 6,17 | 503 | 2,92 |

Totales: **Indicado 13 596 t a 6,83** y **Inferido 2 368 t a 6,78** (in situ); recuperado 17 223 t y
2 907 t tras dilución y 90 % de recuperación.

> **Por qué ningún bloque es «Medido».** El bloque principal tiene sus 3 lados muestreados, pero su
> distancia media a las muestras es de 19,6 m y la regla de ejemplo *Medido* exige 15 m o menos. Lo dice
> el campo `why`: `Medido: dist 19.6 > 15`. Pruebe a cambiar la distancia de *Medido* a 20 en la tabla de
> categorías (o `MIN_VN_RULES rules=[["Medido",3,20,"",""],…]`) y vuelva a cubicar: pasa a Medido.
> **Las reglas del ejemplo no son una norma**: use las de su empresa (guía de vetas, sección de reglas).

## Paso 4 — Probar variantes

- **Ancho mínimo de minado:** súbalo a 1,2 m y cubique de nuevo: aumentan las toneladas diluidas y baja
  la ley diluida en los bloques angostos (`L1-L2/<R1`, potencia media 0,42 m).
- **Excluir una zona:** *Longitudinal* (**MIN_VN_LONG**), dibuje una polilínea cerrada sobre una parte,
  *Edición* = *Excluir prismas* y **Editar en la longitudinal**: esos prismas desaparecen y el modelo se
  recalcula. *Borrar ediciones y límite* lo devuelve al original.
- **Colorear por categoría:** en el panel *Modelo*, *Colorear por* = *Categoría (tras cubicar)*.
- **Con un asistente de IA:** pídale «construye la veta EJ1 con los canales del proyecto, revisa los
  diagnósticos y dime las toneladas por categoría». Guía para asistentes: [`docs/AI_QUICKSTART.md`](../../docs/AI_QUICKSTART.md).

Los resultados de este ejemplo se comprueban automáticamente (prueba
`crates/vein_engine/tests/example_vein.rs`), así que las cifras de este documento se mantienen al día.
