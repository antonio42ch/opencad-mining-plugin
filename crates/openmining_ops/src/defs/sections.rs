//! Sections of solids with planes; 3D polylines and 2D sheets.

#[allow(unused_imports)]
use crate::{input, opt, setting, Group, Item, Kind, OpDef, OpKind, PanelSpec, Param, L, TABLE_FILES};

#[allow(dead_code)]
const FILES: Kind = Kind::Path {
    extensions: TABLE_FILES,
};

const SC_RUN: &[Param] = &[
    setting(
        "mode",
        Kind::Choice {
            options: &[
                opt(
                    "axis",
                    L::new(
                        "Perpendiculares a un eje",
                        "Perpendicular to an axis",
                        "Perpendiculares a um eixo",
                    ),
                ),
                opt(
                    "two_points",
                    L::new(
                        "Verticales por 2 puntos",
                        "Vertical through 2 points",
                        "Verticais por 2 pontos",
                    ),
                ),
                opt(
                    "levels",
                    L::new("Horizontales (niveles)", "Horizontal (levels)", "Horizontais (níveis)"),
                ),
                opt(
                    "three_points",
                    L::new("Por 3 puntos", "Through 3 points", "Por 3 pontos"),
                ),
            ],
        },
        "/sc/mode",
        L::new("Planos", "Planes", "Planos"),
        "How the planes are defined.",
    ),
    setting(
        "interval",
        Kind::num(Some(0.001), None, 1.0),
        "/sc/interval",
        L::new("Intervalo (m)", "Interval (m)", "Intervalo (m)"),
        "Spacing of the planes along the axis (mode axis, m).",
    ),
    setting(
        "ch_from",
        Kind::num(Some(0.0), None, 10.0),
        "/sc/ch_from",
        L::new("PK inicial", "Start chainage", "Estaca inicial"),
        "First chainage (mode axis, m).",
    ),
    setting(
        "ch_to",
        Kind::num(Some(0.0), None, 10.0),
        "/sc/ch_to",
        L::new(
            "PK final (0 = fin del eje)",
            "End chainage (0 = axis end)",
            "Estaca final (0 = fim do eixo)",
        ),
        "Last chainage (mode axis, m; 0 = axis end).",
    ),
    setting(
        "spacing",
        Kind::num(Some(0.001), None, 1.0),
        "/sc/spacing",
        L::new("Espaciado (m)", "Spacing (m)", "Espaçamento (m)"),
        "Distance between parallel planes (mode two_points, m).",
    ),
    setting(
        "count",
        Kind::num(Some(1.0), None, 1.0),
        "/sc/count",
        L::new("Cantidad", "Count", "Quantidade"),
        "Number of parallel planes (mode two_points).",
    ),
    setting(
        "z_from",
        Kind::num(None, None, 5.0),
        "/sc/z_from",
        L::new("Cota inicial", "Start elevation", "Cota inicial"),
        "Lowest level (mode levels).",
    ),
    setting(
        "z_to",
        Kind::num(None, None, 5.0),
        "/sc/z_to",
        L::new("Cota final", "End elevation", "Cota final"),
        "Highest level (mode levels).",
    ),
    setting(
        "z_step",
        Kind::num(Some(0.001), None, 1.0),
        "/sc/z_step",
        L::new("Intervalo de cota", "Elevation interval", "Intervalo de cota"),
        "Level spacing (mode levels, m).",
    ),
    setting(
        "draw_3d",
        Kind::Bool,
        "/sc/draw_3d",
        L::new(
            "Polilíneas 3D en sitio",
            "3D polylines in place",
            "Polilinhas 3D no local",
        ),
        "Draw the sections in place (3D polylines).",
    ),
    setting(
        "draw_sheets",
        Kind::Bool,
        "/sc/draw_sheets",
        L::new("Láminas 2D", "2D sheets", "Folhas 2D"),
        "Draw 2D sheets (one per plane) with grid, title and area.",
    ),
    setting(
        "columns",
        Kind::num(Some(1.0), None, 1.0),
        "/sc/columns",
        L::new("Láminas por fila", "Sheets per row", "Folhas por linha"),
        "Sheets per row.",
    ),
    setting(
        "gap",
        Kind::num(Some(0.0), None, 1.0),
        "/sc/gap",
        L::new(
            "Separación entre láminas",
            "Gap between sheets",
            "Separação entre folhas",
        ),
        "Gap between sheets.",
    ),
    setting(
        "grid",
        Kind::num(Some(0.01), None, 1.0),
        "/sc/grid",
        L::new("Paso de cuadrícula", "Grid spacing", "Passo da grade"),
        "Grid spacing on the sheets (m).",
    ),
    setting(
        "text_height",
        Kind::num(Some(0.01), None, 0.1),
        "/sc/text_height",
        L::new("Altura de texto", "Text height", "Altura do texto"),
        "Text height on the sheets.",
    ),
    setting(
        "auto_place",
        Kind::Bool,
        "/sc/auto_place",
        L::new(
            "Ubicar láminas a la derecha de los sólidos",
            "Place sheets to the right of the solids",
            "Posicionar folhas à direita dos sólidos",
        ),
        "Place the sheets automatically beside the drawing extents.",
    ),
    setting(
        "sheet_x",
        Kind::num(None, None, 10.0),
        "/sc/sheet_x",
        L::new("Láminas: X inicial", "Sheets: start X", "Folhas: X inicial"),
        "Sheet insertion X (auto_place off).",
    ),
    setting(
        "sheet_y",
        Kind::num(None, None, 10.0),
        "/sc/sheet_y",
        L::new("Láminas: Y inicial", "Sheets: start Y", "Folhas: Y inicial"),
        "Sheet insertion Y (auto_place off).",
    ),
    setting(
        "replace",
        Kind::Bool,
        "/sc/replace",
        L::new(
            "Reemplazar secciones anteriores",
            "Replace previous sections",
            "Substituir seções anteriores",
        ),
        "Replace the sections and sheets drawn before.",
    ),
    input(
        "solids",
        Kind::Handles,
        false,
        L::new("Sólidos", "Solids", "Sólidos"),
        "Meshes to cut. Without it: the selection, or a pick.",
    ),
    input(
        "axis",
        Kind::Handle,
        false,
        L::new("Eje", "Axis", "Eixo"),
        "Axis polyline (mode axis).",
    ),
    input(
        "p1",
        Kind::Point,
        false,
        L::new("Punto 1", "Point 1", "Ponto 1"),
        "First plane point (modes two_points, three_points).",
    ),
    input(
        "p2",
        Kind::Point,
        false,
        L::new("Punto 2", "Point 2", "Ponto 2"),
        "Second plane point.",
    ),
    input(
        "p3",
        Kind::Point,
        false,
        L::new("Punto 3", "Point 3", "Ponto 3"),
        "Third plane point (mode three_points).",
    ),
];

pub static OPS: &[OpDef] = &[
OpDef { name: "sc_run", command: "MIN_SC_RUN", aliases: &[],
            label: L::new("Generar secciones", "Generate sections", "Gerar seções"), group: Group::Sections, kind: OpKind::Action,
            summary: "Cut solids (meshes) with planes: perpendicular to an axis every interval, vertical through 2 points (+ parallels), horizontal levels, or through 3 points. Draws 3D polylines in place and/or 2D sheets with grid, title and area.", params: SC_RUN,
            positional: &[], result: "`planes`, `cut` (planes that intersect), `polylines`, `sheets`." },
];

pub static PANELS: &[PanelSpec] = &[PanelSpec {
    id: "min_sections",
    title: L::new("Secciones", "Sections", "Seções"),
    command: "MIN_SC_PANEL",
    items: &[
        Item::Section {
            id: "sc_planes",
            title: L::new("Planos de corte", "Cutting planes", "Planos de corte"),
            items: &[
                Item::Param("sc_run", "mode"),
                Item::Param("sc_run", "interval"),
                Item::Param("sc_run", "ch_from"),
                Item::Param("sc_run", "ch_to"),
                Item::Param("sc_run", "spacing"),
                Item::Param("sc_run", "count"),
                Item::Param("sc_run", "z_from"),
                Item::Param("sc_run", "z_to"),
                Item::Param("sc_run", "z_step"),
            ],
            collapsed: false,
        },
        Item::Section {
            id: "sc_output",
            title: L::new("Resultado", "Output", "Resultado"),
            items: &[
                Item::Param("sc_run", "draw_3d"),
                Item::Param("sc_run", "draw_sheets"),
                Item::Param("sc_run", "columns"),
                Item::Param("sc_run", "gap"),
                Item::Param("sc_run", "grid"),
                Item::Param("sc_run", "text_height"),
                Item::Param("sc_run", "auto_place"),
                Item::Param("sc_run", "sheet_x"),
                Item::Param("sc_run", "sheet_y"),
                Item::Param("sc_run", "replace"),
            ],
            collapsed: false,
        },
        Item::Info(L::new(
            "Seleccione los sólidos (mallas) antes o durante el comando.",
            "Select the solids (meshes) before or during the command.",
            "Selecione os sólidos (malhas) antes ou durante o comando.",
        )),
        Item::Button("sc_run"),
    ],
}];
