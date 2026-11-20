//! Geological mapping: traces and projection along the dip.

#[allow(unused_imports)]
use crate::{input, opt, setting, Group, Item, Kind, OpDef, OpKind, PanelSpec, Param, L, TABLE_FILES};

#[allow(dead_code)]
const FILES: Kind = Kind::Path {
    extensions: TABLE_FILES,
};

const GM_TRACE: &[Param] = &[
    setting(
        "e",
        Kind::num(None, None, 1.0),
        "/gm/e",
        L::new("Este", "Easting", "Leste"),
        "East of the anchor point (world).",
    ),
    setting(
        "n",
        Kind::num(None, None, 1.0),
        "/gm/n",
        L::new("Norte", "Northing", "Norte"),
        "North of the anchor point.",
    ),
    setting(
        "z",
        Kind::num(None, None, 1.0),
        "/gm/z",
        L::new("Cota", "Elevation", "Cota"),
        "Elevation of the anchor point.",
    ),
    setting(
        "bearing",
        Kind::Text,
        "/gm/bearing",
        L::new("Rumbo / azimut", "Bearing / azimuth", "Rumo / azimute"),
        "Azimuth in degrees or bearing (N45E, S30W, N10O).",
    ),
    setting(
        "length",
        Kind::num(Some(0.01), None, 0.5),
        "/gm/length",
        L::new("Longitud", "Length", "Comprimento"),
        "Trace length (m).",
    ),
    setting(
        "anchor",
        Kind::Choice {
            options: &[
                opt("start", L::new("Inicio", "Start", "Início")),
                opt("centre", L::new("Centro", "Centre", "Centro")),
                opt("end", L::new("Fin", "End", "Fim")),
            ],
        },
        "/gm/anchor",
        L::new("Punto de inserción", "Insertion point", "Ponto de inserção"),
        "Which point of the trace the coordinates give.",
    ),
    setting(
        "dip",
        Kind::Text,
        "/gm/dip",
        L::new("Buzamiento (0–90)", "Dip (0–90)", "Mergulho (0–90)"),
        "Dip of the mapped structure in degrees (0-90); stored with the trace. Empty = none.",
    ),
    setting(
        "dip_dir",
        Kind::Choice {
            options: &[
                opt(
                    "right",
                    L::new(
                        "A la derecha de la traza",
                        "To the right of the trace",
                        "À direita do traço",
                    ),
                ),
                opt(
                    "left",
                    L::new(
                        "A la izquierda de la traza",
                        "To the left of the trace",
                        "À esquerda do traço",
                    ),
                ),
                opt("azimuth", L::new("Azimut fijo", "Fixed azimuth", "Azimute fixo")),
            ],
        },
        "/gm/dip_dir",
        L::new("Sentido del buzamiento", "Dip direction", "Sentido do mergulho"),
        "Dip direction: to the right/left of the drawing direction, or a fixed azimuth (dip_az).",
    ),
    setting(
        "dip_az",
        Kind::Text,
        "/gm/dip_az",
        L::new(
            "Azimut / rumbo del buzamiento (si es fijo)",
            "Dip azimuth / bearing (if fixed)",
            "Azimute / rumo do mergulho (se fixo)",
        ),
        "Fixed dip direction (azimuth or bearing), with dip_dir = azimuth.",
    ),
    setting(
        "rock_type",
        Kind::Text,
        "/gm/rock_type",
        L::new("Tipo de roca", "Rock type", "Tipo de rocha"),
        "Rock type (stored in the trace XDATA).",
    ),
    setting(
        "thickness",
        Kind::Text,
        "/gm/thickness",
        L::new("Potencia", "Thickness", "Espessura"),
        "Thickness.",
    ),
    setting(
        "mineralization",
        Kind::Text,
        "/gm/mineralization",
        L::new("Mineralización", "Mineralisation", "Mineralização"),
        "Mineralization.",
    ),
    setting(
        "alteration",
        Kind::Text,
        "/gm/alteration",
        L::new("Alteración", "Alteration", "Alteração"),
        "Alteration.",
    ),
    setting(
        "structure",
        Kind::Text,
        "/gm/structure",
        L::new("Estructura", "Structure", "Estrutura"),
        "Structure type.",
    ),
    setting(
        "grade",
        Kind::Text,
        "/gm/grade",
        L::new("Ley", "Grade", "Teor"),
        "Grade observed.",
    ),
    setting(
        "sample",
        Kind::Text,
        "/gm/sample",
        L::new("Código de muestra", "Sample code", "Código da amostra"),
        "Sample id.",
    ),
];

const GM_TRACE_PICK: &[Param] = &[input(
    "at",
    Kind::Point,
    false,
    L::new("Punto", "Point", "Ponto"),
    "Anchor point x,y,z. Without it a pick is started.",
)];

const GM_PROJECT: &[Param] = &[
    setting(
        "dip",
        Kind::Text,
        "/gm/dip",
        L::new("Buzamiento (0–90)", "Dip (0–90)", "Mergulho (0–90)"),
        "Dip of the mapped structure in degrees (0-90); stored with the trace. Empty = none.",
    ),
    setting(
        "dip_dir",
        Kind::Choice {
            options: &[
                opt(
                    "right",
                    L::new(
                        "A la derecha de la traza",
                        "To the right of the trace",
                        "À direita do traço",
                    ),
                ),
                opt(
                    "left",
                    L::new(
                        "A la izquierda de la traza",
                        "To the left of the trace",
                        "À esquerda do traço",
                    ),
                ),
                opt("azimuth", L::new("Azimut fijo", "Fixed azimuth", "Azimute fixo")),
            ],
        },
        "/gm/dip_dir",
        L::new("Sentido del buzamiento", "Dip direction", "Sentido do mergulho"),
        "Dip direction: to the right/left of the drawing direction, or a fixed azimuth (dip_az).",
    ),
    setting(
        "dip_az",
        Kind::Text,
        "/gm/dip_az",
        L::new(
            "Azimut / rumbo del buzamiento (si es fijo)",
            "Dip azimuth / bearing (if fixed)",
            "Azimute / rumo do mergulho (se fixo)",
        ),
        "Fixed dip direction (azimuth or bearing), with dip_dir = azimuth.",
    ),
    setting(
        "proj_mode",
        Kind::Choice {
            options: &[
                opt(
                    "height",
                    L::new("Altura de destino", "Target height", "Altura de destino"),
                ),
                opt(
                    "delta",
                    L::new(
                        "Diferencia de altura (+ sube, − baja)",
                        "Height difference (+ up, − down)",
                        "Diferença de altura (+ sobe, − desce)",
                    ),
                ),
            ],
        },
        "/gm/proj_mode",
        L::new("Definir la altura por", "Define the height by", "Definir a altura por"),
        "Target height, or height difference (+ up, - down).",
    ),
    setting(
        "proj_value",
        Kind::num(None, None, 1.0),
        "/gm/proj_value",
        L::new("Valor", "Value", "Valor"),
        "The target height or the difference (m).",
    ),
    input(
        "traces",
        Kind::Handles,
        false,
        L::new("Trazas", "Traces", "Traços"),
        "Lines / polylines to project. Without it: the selection, or a pick.",
    ),
];

pub static OPS: &[OpDef] = &[
OpDef { name: "gm_trace", command: "MIN_GM_TRACE", aliases: &[],
            label: L::new("Dibujar traza", "Draw trace", "Desenhar traço"), group: Group::Geology, kind: OpKind::Action,
            summary: "Draw a geological trace (structure, vein, contact) from coordinates, bearing/azimuth and length; dip and observations are stored in its XDATA.", params: GM_TRACE,
            positional: &[], result: "`trace` (id), `azimuth`, `bearing`, `start`, `end` (`[x,y,z]`)." },
OpDef { name: "gm_trace_pick", command: "MIN_GM_TRACE_PICK", aliases: &[],
            label: L::new("Trazar con clic", "Trace by click", "Traçar com clique"), group: Group::Geology, kind: OpKind::Action,
            summary: "Same as gm_trace at a given point (fills e, n, z); without the point a pick with live preview is started.", params: GM_TRACE_PICK,
            positional: &[], result: "as gm_trace." },
OpDef { name: "gm_project", command: "MIN_GM_PROJECT", aliases: &[],
            label: L::new("Proyectar trazas seleccionadas", "Project selected traces", "Projetar traços selecionados"), group: Group::Geology, kind: OpKind::Action,
            summary: "Project mapped lines/polylines to another height along their dip (to a target height or by a height difference), down-dip by dz / tan(dip). The dip comes from the arguments/panel, or from the one stored with each trace when dip is empty.", params: GM_PROJECT,
            positional: &[], result: "`projected` (traces), `skipped` (with reasons), `shift` (horizontal displacement of the first vertex, m)." },
];

pub static PANELS: &[PanelSpec] = &[PanelSpec {
    id: "min_geomap",
    title: L::new("Mapeo geológico", "Geological mapping", "Mapeamento geológico"),
    command: "MIN_GM_PANEL",
    items: &[
        Item::Section {
            id: "gm_trace",
            title: L::new("Traza", "Trace", "Traço"),
            items: &[
                Item::Param("gm_trace", "e"),
                Item::Param("gm_trace", "n"),
                Item::Param("gm_trace", "z"),
                Item::Param("gm_trace", "bearing"),
                Item::Param("gm_trace", "length"),
                Item::Param("gm_trace", "anchor"),
                Item::Row(&[Item::Button("gm_trace"), Item::Button("gm_trace_pick")]),
            ],
            collapsed: false,
        },
        Item::Section {
            id: "gm_dipf",
            title: L::new("Buzamiento", "Dip", "Mergulho"),
            items: &[
                Item::Param("gm_trace", "dip"),
                Item::Param("gm_trace", "dip_dir"),
                Item::Param("gm_trace", "dip_az"),
            ],
            collapsed: false,
        },
        Item::Section {
            id: "gm_proj",
            title: L::new(
                "Proyección por buzamiento",
                "Projection along the dip",
                "Projeção pelo mergulho",
            ),
            items: &[
                Item::Param("gm_project", "proj_mode"),
                Item::Param("gm_project", "proj_value"),
                Item::Button("gm_project"),
            ],
            collapsed: false,
        },
        Item::Section {
            id: "gm_obs",
            title: L::new("Observación", "Observation", "Observação"),
            items: &[
                Item::Param("gm_trace", "rock_type"),
                Item::Param("gm_trace", "thickness"),
                Item::Param("gm_trace", "mineralization"),
                Item::Param("gm_trace", "alteration"),
                Item::Param("gm_trace", "structure"),
                Item::Param("gm_trace", "grade"),
                Item::Param("gm_trace", "sample"),
            ],
            collapsed: true,
        },
    ],
}];
