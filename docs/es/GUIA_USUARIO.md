# Open Mining — guía de usuario

[English](../GUIDE.md) · [Instalar](INSTALAR.md) · [Comandos](COMANDOS.md) · [Vetas angostas](VETAS.md)

Cómo usar cada módulo desde la cinta. Cada botón ejecuta un comando (`MIN_…`); los mismos comandos se pueden
escribir en la línea de comandos o enviarlos un asistente de IA (vea [AI_QUICKSTART.md](../AI_QUICKSTART.md),
en inglés). Datos para practicar: [examples/](../../examples).

## Cómo funciona

- **Pestaña y paneles.** La cinta tiene la pestaña **Minería** con un grupo por módulo. El botón grande abre el
  **panel** del módulo; los pequeños ejecutan una acción con los valores del panel.
- **La configuración vive en el dibujo.** Las rutas, opciones y tablas que fije en un panel se guardan en el DWG
  y vuelven al abrirlo.
- **Una acción, un deshacer.** Ctrl+Z deshace un dibujo o cálculo completo. Al volver a dibujar se reemplaza lo
  anterior: no hay duplicados.
- **Todo lo que dibuja el plugin está etiquetado** (XDATA `OPENMINING`): se puede consultar, redibujar o borrar.
- **Archivos junto al dibujo.** Las tablas que produce (canales convertidos, compósitos, prismas, bloques…) se
  escriben como `<dibujo>_….csv`; guarde antes el dibujo.
- **Coordenadas.** Los datos están en coordenadas reales. Con un *origen local* activado en el panel de Sondajes,
  lo dibujado se desplaza por él (para trabajar con números pequeños); el plugin lo revierte al leer el dibujo.
- **Tablas.** CSV, TXT, TSV, XLSX, XLS, ODS; se reconocen separadores `;` o `,` y coma decimal. Las columnas se
  reconocen por alias (español, inglés, portugués); si alguna no se reconoce, un diálogo de mapeo pregunta una
  vez y lo recuerda.
- **Idiomas.** Español, inglés y portugués; vea [INSTALAR.md](INSTALAR.md#idioma).

## 1. Sondajes

Grupo **Sondajes** → **Panel**.

1. Elija las tablas de collar, survey, ensayos y (opcional) geología → **Cargar**. La línea de estado cuenta los
   collares y los mensajes de validación (reglas R01–R07: collares faltantes, sondajes duplicados, conflictos de
   profundidad…).
2. Elija la ley por la que colorear (atributo de ensayo) y el esquema de colores (azul→rojo, arcoíris, rangos
   propios) o un atributo de geología con su tabla de colores (**Generar tabla de colores** la llena con los datos).
3. **Dibujar** → trazas 3D (desurvey por mínima curvatura) en una capa por atributo, con rótulos. Al dibujar de
   nuevo se reemplaza lo anterior.
4. **Borrar** quita todo lo que dibujó el módulo.

Por comando: `MIN_DH_LOAD collar=… survey=… assays=…`, `MIN_DH_DRAW assay_attr=au`, `MIN_DH_LIST hole=…`.

## 2. Canales de muestreo

Grupo **Sondajes** → **Canales**. El muestreo sistemático dentro de las labores se convierte en **datos tipo
sondaje**, así que todas las herramientas de sondajes (dibujo, compósitos, estimación) y el modelo de veta los
usan.

- Tabla de **puntos topográficos**: `PUNTO; ESTE; NORTE; COTA`.
- **Hoja de muestreo**, una fila por muestra: canal, punto topográfico, distancia (m) desde el punto a lo largo
  de la labor, ancho de la muestra (o desde / hasta) y, opcionalmente, la dirección de la labor (*hacia* el
  siguiente punto o un azimut), *potencia* (espesor real de la veta), tipo de muestra (`VETA`, `CAJA TECHO`…),
  labor, id de muestra, posición y **cualquier número de columnas de ley** (`AU`, `AG`, `CU%`…). Se aceptan
  límites de detección como `<0,01`.
- **Convertir canales** → los sondajes `CH-<canal>` se agregan a los sondajes cargados. Por defecto cada canal
  es **horizontal y perpendicular** a la labor, centrado en su posición. Un canal puede traer su propio azimut /
  inclinación en la hoja, u orientarse en pantalla con **Orientar canal en pantalla** (dos clics); la
  orientación queda guardada en una tabla.
- Con *Reconvertir al cargar sondajes* los canales se vuelven a agregar tras cada carga de sondajes.

Recorrido con datos: [examples/canales](../../examples/canales/README.md). Por comando:
`MIN_CH_CONVERT points=… sheet=…`, `MIN_CH_ORIENT channel=… p1=… p2=…`.

## 3. Vetas angostas

Grupo **Vetas** (superficies → prismas y leyes → cubicación). Una veta angosta **no** se modela con un modelo de
bloques regular: el plugin construye superficies para las cajas y prismas 3D entre ellas. Descripción completa
con las reglas y todos los parámetros: **[VETAS.md](VETAS.md)**; ejemplo con las cifras que debe obtener:
**[examples/veta](../../examples/veta/README.md)**.

1. **Veta: superficies** — techo y piso desde polilíneas de contacto mapeadas (por capa o elegidas) y desde los
   tramos de veta de sondajes y canales. Revise los diagnósticos (cruces, peores contactos).
2. **Veta: modelo de prismas** — variograma en el plano de *ley × potencia*, prismas con ley y potencia por
   kriging, validación contra las muestras, longitudinal; dibuje un polígono sobre la longitudinal para excluir
   prismas, fijar su ley o potencia, o limitar el modelo.
3. **Veta: cubicación** — dilución (ancho mínimo de minado, sobre-excavación con su ley) y recuperación; bloques
   entre niveles y chimeneas; lados muestreados; **categorías por reglas** que usted define en una tabla.
4. **Construir todo** ejecuta las tres etapas en un solo paso.

> Las reglas de categoría que trae la tabla son **de ejemplo**, no una norma. Reemplácelas por la norma de su
> empresa y pruébelas con `MIN_VN_RULES` antes de cubicar.

## 4. Labores, ejes y secciones

- **Ejes**: punto inicial (E, N, Z) y azimut; luego **Agregar recta** (longitud, pendiente en % o °) y **Agregar
  curva** (radio + ángulo o longitud, derecha / izquierda); cada elemento continúa la misma polilínea 3D.
  **Quitar último** y **Nuevo eje** editan la cadena; **Continuar polilínea** recupera un eje de un dibujo guardado.
- **Labores**: **Por eje** crea un sólido cerrado (malla) a lo largo de un eje con una sección (rectángulo,
  herradura…); **Contornos H / V** lo crean desde cuatro polilíneas. El volumen se imprime en
  la línea de comandos.
- **Secciones**: cortan los sólidos con planos perpendiculares a un eje, verticales por dos puntos, niveles
  horizontales o tres puntos; polilíneas 3D en su lugar y planos 2D con cuadrícula, título y área.

## 5. Mapeo geológico

Grupo **Geología**. **Traza**: E / N / Z + rumbo (`N45E`, `S30W` o un azimut) + longitud y ancla dibujan una línea
en `MIN_GM_TRAZA` (los campos opcionales de roca / potencia / ley quedan en su XDATA). **Trazar con clic** la
previsualiza en vivo. **Proyectar** lleva las trazas seleccionadas a otra altura siguiendo su buzamiento.

## 6. Estimación de recursos

Grupo **Recursos** — detalles y formatos de archivo en [RESOURCES.md](../RESOURCES.md) (inglés).

1. Panel **Compósitos**: defina los *dominios* (nombre, valores de geología, densidad, tope, superficie de contacto
   o sólidos opcionales) → **Compositar y analizar**: estadísticas e histogramas por dominio;
   `<dibujo>_composites.csv`.
2. Panel **Variografía**: elija un dominio, calcule los variogramas en los tres ejes, **Autoajustar**, guarde el
   modelo en el dominio, valide con validación cruzada.
3. Panel **Modelo**: tamaño de bloque, rotación y búsqueda → **Estimar** (kriging ordinario / simple, IDW o vecino
   más cercano) → recursos por dominio; el modelo se guarda como `<dibujo>.ombm`.
4. **Validar** (swath, curva ley-tonelaje, histograma) y **Dibujar vista** (niveles, secciones por dos puntos o a lo
   largo de un eje, cubos sobre la ley de corte).

## 7. Geotecnia

Grupo **Geotecnia** — fórmulas y tablas en [GEOTECH.md](../GEOTECH.md) (inglés); datos para practicar:
[examples/geotechnics](../../examples/geotechnics/README.md).

- **Datos**: cargue las tablas de logueo geotécnico, laboratorio y mapeo → **Clasificar y dibujar**: RMR89, Q y GSI
  por intervalo, coloreados por clase. Los datos faltantes se informan, nunca se rellenan en silencio.
- **Hoek-Brown**: GSI, *mi*, D, σci (o **Usar datos clasificados**) → envolvente de resistencia y módulo del macizo.
- **Modelo**: estime los atributos geotécnicos en el modelo de bloques (con sus columnas de confianza) y **Ver
  modelo**: por clase, con filtro por umbral y corte por un plano, en plantas, secciones o cubos.
- **Estructuras y diseño**: estereogramas y familias de discontinuidades, análisis cinemático, cuñas; esfuerzos
  in situ, Kirsch, convergencia–confinamiento con soporte, pilares y pilares de corona.

## Pedírselo a un asistente de IA

Con el asistente conectado (vea [AI_QUICKSTART.md](../AI_QUICKSTART.md)), puede pedirle en lenguaje normal, por
ejemplo:

- «Carga los sondajes de esta carpeta y dime si la validación encontró problemas.»
- «Convierte la hoja de canales y los puntos topográficos, y muéstrame las leyes a lo largo del nivel 3950.»
- «Construye la veta V1 con los canales, revisa los diagnósticos y dime qué cambiarías.»
- «¿Por qué el bloque L1-L2/>R1 no es *Medido*?»  (el asistente lee el `why` del bloque y la tabla de reglas)
- «Sube el ancho mínimo de minado a 1,2 m y compara las toneladas con el modelo actual.»

El asistente usa las mismas operaciones que los botones, de modo que usted ve y puede deshacer todo lo que hace.
