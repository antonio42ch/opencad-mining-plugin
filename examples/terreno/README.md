# Ejemplo: secciones con terreno

Un terreno pequeño (una superficie TIN de las herramientas **Civil** de OpenCAD Studio) y las secciones del
plugin cortándolo: el perfil del terreno en las secciones verticales y las curvas de nivel en las plantas.
Necesita OpenCAD Studio v2026.47 o superior.

## 1. Crear el terreno

Documento nuevo. En la línea de comandos (o con **Civil → Puntos** y **Civil → Superficie**):

```
CREATEPOINTS 0,0,100 50,0,102 100,0,101 0,50,103 50,50,105 100,50,104 0,100,102 50,100,104 100,100,103
```

Seleccione los 9 puntos COGO que se crearon y ejecute `CREATESURFACE`: al pedir las líneas de ruptura y el
límite exterior, pulse Enter (ninguno). Resultado: la superficie `Surface 1`, de 9 vértices y 8 triángulos, con
cotas de 100 a 105 m.

## 2. Ver los terrenos

Panel **Secciones** → sección **Terreno** → **Buscar terrenos** (o `MIN_TR_LIST`):

```
Terrenos en el dibujo: Surface 1.
```

El resultado trae el nombre, el *handle*, los triángulos y el rango de cotas; el nombre es lo que se elige en la
lista *Terreno* del panel.

## 3. Perfil del terreno en una sección vertical

Panel **Secciones**: *Planos* = *Verticales por 2 puntos*, *Terreno* = `Surface 1`, y los dos puntos
`0,50,0` y `100,50,0` (por comando:
`MIN_SC_RUN mode=two_points p1=0,50,0 p2=100,50,0 count=1 terrain="Surface 1"`).

Esperado: un plano que corta el terreno, una polilínea (el perfil a lo largo de y = 50) con cotas de **103 a 105 m**,
dibujada en 3D en su lugar y en una lámina 2D con la etiqueta `Terreno: 103.0 – 105.0`, en la capa
`MIN_SC_TERRENO` (verde).

## 4. Curvas de nivel en plantas

*Planos* = *Horizontales (niveles)*, cota inicial 101, final 104, paso 1
(`MIN_SC_RUN mode=levels z_from=101 z_to=104 z_step=1 terrain="Surface 1"`).

Esperado: 4 planos y 4 curvas de nivel del terreno (101, 102, 103 y 104 m), cada una en su lámina.

## Con sólidos

Con sólidos seleccionados (galerías, caserones) las secciones los cortan como siempre y el terreno se dibuja
sobre la **extensión de los sólidos en ese plano más el margen** (*Margen del terreno*, 20 m por defecto), para
que la lámina no sea tan ancha como el terreno; en las verticales el recorte es solo a lo largo de la sección,
nunca en altura, de modo que se ve toda la cobertura sobre la labor. Sin sólidos seleccionados y con un terreno
elegido, solo se corta el terreno (con un ancho de *Ancho del terreno sin sólidos*, 100 m a cada lado, en las
verticales).

## Con un asistente de IA

```
MIN_TR_LIST
MIN_SC_RUN mode=axis axis=<handle del eje> solids=<h1>,<h2> interval=10 terrain="Surface 1"
```

El resultado de `sc_run` trae `terrain`: `planes_cut`, `polylines`, `min_elevation`, `max_elevation` y `missed`
(los planos que el terreno no cruza). Guía para asistentes: [`docs/AI_QUICKSTART.md`](../../docs/AI_QUICKSTART.md).

Los resultados de este ejemplo se comprueban automáticamente en la prueba contra OpenCAD Studio de la CI.
