# Ejemplo de canales de muestreo

Una veta de rumbo N30E muestreada en dos galerías (niveles 3950 y 4000). Cada canal tiene tres
muestras en el techo de la galería: caja techo, veta y caja piso.

| Archivo | Contenido |
|---|---|
| `puntos_topograficos.csv` | 7 puntos topográficos (PUNTO; ESTE; NORTE; COTA) |
| `muestreo_canales.csv` | 24 canales, 72 muestras: CANAL; LABOR; PUNTO; HACIA; DISTANCIA; POSICION; ANCHO; POTENCIA; TIPO; MUESTRA; AU; AG |

## Formato de la hoja de muestreo

Una fila por muestra. Columnas reconocidas (mayúsculas, tildes y espacios no importan):

- **Obligatorias:** canal, punto (topográfico), distancia (m desde el punto), y ancho de la
  muestra (o desde y hasta).
- **Dirección de la labor:** hacia (el siguiente punto) o azimut.
- **Opcionales:** potencia, tipo (VETA, CAJA TECHO, CAJA PISO…), labor, muestra, posición,
  altura, azimut e inclinación propios del canal (az_canal, inc_canal).
- **Leyes:** cualquier otra columna numérica (AU, AG, CU%…). Los límites de detección como
  `<0,01` se aceptan. Las columnas de texto se guardan como geología.

El canal se ubica en el punto topográfico más la distancia en la dirección de la labor; por defecto es
horizontal, perpendicular a la labor (cruza la veta cuando la galería va sobre ella) y centrado en esa
posición. Cada canal puede traer su propia orientación, o se orienta en pantalla con
**Orientar canal en pantalla** (dos clics).

## Pasos

1. Sondajes → **Canales** (panel): elija `puntos_topograficos.csv` y `muestreo_canales.csv` →
   **Convertir canales**. Esperado: `Canales: 24 convertidos (72 muestras)`.
2. Sondajes → **Dibujar** con atributo de ley `au`: los canales aparecen como tramos cortos en el techo de
   las galerías, coloreados por ley. Con geología `tipo` se distinguen veta y cajas.
3. Orientar un canal: en el panel escriba el canal (p. ej. `3950-101-03`) → **Orientar canal en pantalla** y
   dé dos clics; el canal se reconvierte con esa dirección y queda en la tabla de orientaciones.
4. Por comando:
   ```
   MIN_CH_CONVERT points="<carpeta>/puntos_topograficos.csv" sheet="<carpeta>/muestreo_canales.csv"
   MIN_DH_DRAW assay_attr=au
   MIN_CH_ORIENT channel=3950-101-03 p1=0,0,0 p2=1,1,0
   ```
5. Los canales quedan como sondajes `CH-…`: compósitos, consultas (`MIN_DH_LIST hole=CH-3950-101-03`) y,
   en la próxima etapa, el modelo de veta los usan. Si el dibujo está guardado se escriben también
   `<dibujo>_canales_collar.csv`, `_survey.csv`, `_leyes.csv` y `_geo.csv`.
