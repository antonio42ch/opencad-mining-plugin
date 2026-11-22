# Open Mining — hoja rápida

[English](../COMMANDS.md) · [Guía de usuario](GUIA_USUARIO.md) · referencia completa con argumentos y resultados: [MCP.md](../MCP.md)

Cada comando se puede escribir en la línea de comandos como `MIN_X clave=valor …`, ejecutar desde su botón de la
cinta o enviarlo un asistente de IA. `MIN_HELP` los lista; `MIN_HELP op=<nombre>` describe uno; `MIN_STATUS`
indica qué hacer a continuación.

| Grupo de la pestaña | Comandos | Qué hacen |
|---|---|---|
| **Proyecto** | `MIN_STATUS` `MIN_HELP` `MIN_ENTITIES` | estado y próximos pasos, ayuda de operaciones, entidades que dibujó el plugin |
| **Sondajes** | `MIN_DH_PANEL` `MIN_DH_LOAD` `MIN_DH_LIST` `MIN_DH_DRAW` `MIN_DH_CLEAR` `MIN_DH_GEOCOLORS` | cargar tablas CSV/XLSX, validar (R01–R07), listar, dibujar trazas 3D por ley o geología, borrar, generar la tabla de colores de geología |
| **Canales** | `MIN_CH_PANEL` `MIN_CH_CONVERT` `MIN_CH_ORIENT` | hojas de muestreo de canales → datos tipo sondaje `CH-…`; orientar un canal (dos clics o `p1=` `p2=`) |
| **Vetas — superficies** | `MIN_VN_PANEL` `MIN_VN_SURFACES` | superficies de techo y piso desde contactos e intersecciones, con diagnósticos |
| **Vetas — modelo** | `MIN_VN_MODEL_PANEL` `MIN_VN_VARIOGRAM` `MIN_VN_ESTIMATE` `MIN_VN_LONG` `MIN_VN_EDIT` | variograma en el plano, prismas con leyes, validación, longitudinal, ediciones por polígono |
| **Vetas — cubicación** | `MIN_VN_TONNAGE_PANEL` `MIN_VN_CUBICATE` `MIN_VN_RULES` `MIN_VN_BUILD` | dilución y recuperación, bloques entre niveles y chimeneas, categorías por sus reglas; construir todo |
| **Labores** | `MIN_UG_PANEL` `MIN_UG_AXIS` `MIN_UG_HBOUND` `MIN_UG_VBOUND` | sólidos cerrados desde un eje + sección, o desde cuatro polilíneas de contorno |
| **Ejes** | `MIN_AX_PANEL` `MIN_AX_LINE` `MIN_AX_CURVE` `MIN_AX_UNDO` `MIN_AX_NEW` `MIN_AX_PICKSTART` `MIN_AX_FROMPOLY` | ejes de labores como rectas y curvas con pendiente |
| **Secciones** | `MIN_SC_PANEL` `MIN_SC_RUN` | cortar sólidos con planos: perpendiculares a un eje, dos puntos, niveles, tres puntos |
| **Recursos** | `MIN_RS_COMP_PANEL` `MIN_RS_VARIO_PANEL` `MIN_RS_BM_PANEL` `MIN_RS_COMPOSITE` `MIN_RS_STATS` `MIN_RS_VARIO` `MIN_RS_FIT` `MIN_RS_MODELSAVE` `MIN_RS_ESTIMATE` `MIN_BM_RESOURCES` `MIN_BM_SAMPLE` `MIN_RS_BMVIEW` | compósitos, dominios, variogramas, kriging / IDW, modelo de bloques, recursos y vistas — vea [RESOURCES.md](../RESOURCES.md) |
| **Geotecnia** | `MIN_GT_DATA_PANEL` `MIN_GT_HB_PANEL` `MIN_GT_MODEL_PANEL` `MIN_GT_LOAD` `MIN_GT_CLASSIFY` `MIN_GT_HB` `MIN_GT_ESTIMATE` `MIN_GT_VIEW` | RMR89 / Q / GSI, Hoek-Brown, modelo geotécnico de bloques y vistas — vea [GEOTECH.md](../GEOTECH.md) |
| **Estructuras y diseño** | `MIN_RM_STRUCT_PANEL` `MIN_RM_DESIGN_PANEL` `MIN_ST_*` `MIN_SM_*` | estereogramas, familias, cinemática, cuñas; esfuerzos, Kirsch, convergencia, pilares |
| **Geología** | `MIN_GM_PANEL` `MIN_GM_TRACE` `MIN_GM_TRACE_PICK` `MIN_GM_PROJECT` | trazas desde coordenadas + rumbo, proyección según el buzamiento |

## Veta angosta en cuatro líneas

```
MIN_CH_CONVERT points="…/puntos_topograficos.csv" sheet="…/muestreo_canales.csv"
MIN_VN_BUILD vein=V1 vein_attr=tipo vein_values=VETA grade=au thickness_col=potencia levels=<h1>,<h2> raises=<h3>
MIN_VN_RULES sides=2 dist=20
MIN_VN_CUBICATE vein=V1
```

## Parámetros que conviene conocer (vetas)

| Argumento | Por defecto | Significado |
|---|---|---|
| `vein_attr`, `vein_values` | `tipo`, `VETA` | columna de geología y valores que marcan los tramos de veta |
| `grade`, `thickness_col` | `au`, `potencia` | columna de ensayo a estimar, columna de potencia real |
| `hw_layer`, `fw_layer` | `VETA_TECHO`, `VETA_PISO` | capas de las polilíneas de contacto mapeadas |
| `step`, `radius`, `margin` | 2, 50, 5 m | malla de las superficies, radio de búsqueda (= extrapolación máxima), margen |
| `block_u`, `block_v` | 5, 5 m | tamaño del prisma en rumbo y en buzamiento |
| `max_distance` | 40 m | los prismas más lejos de toda muestra se dejan fuera (0 = sin límite) |
| `density` | 2,7 t/m³ | densidad de la veta |
| `mmw`, `overbreak_hw`, `overbreak_fw`, `recovery` | 0,8 m, 0,10 m, 0,05 m, 90 % | dilución y recuperación |
| `level_layer`, `raise_layer` | `NIVELES`, `CHIMENEAS` | capas de niveles y chimeneas |
| `rules` | ejemplos | **reglas de categoría: reemplácelas por su norma** |

## Capas que crea el plugin

`MIN_DH_*`, `MIN_RS_COMP_*`, `MIN_RS_*` (sondajes, compósitos) · `MIN_UG_SOLIDO` (labores) · `MIN_SC_CORTE` (secciones) ·
`MIN_GM_TRAZA`, `MIN_GM_PROYECCION` (geología) · `MIN_GT_*` (geotecnia) · `MIN_VN_TECHO`, `MIN_VN_PISO`,
`MIN_VN_PRISMAS`, `MIN_VN_LONG`, `MIN_VN_BLOQUES` (vetas). Lista completa con colores y clases XDATA:
[PLUGIN.md](../PLUGIN.md).
