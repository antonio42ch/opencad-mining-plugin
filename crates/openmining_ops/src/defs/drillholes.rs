//! Drillholes: import, validation, desurvey and drawing.

#[allow(unused_imports)]
use crate::{input, opt, setting, Group, Item, Kind, OpDef, OpKind, PanelSpec, Param, L, TABLE_FILES};

#[allow(dead_code)]
const FILES: Kind = Kind::Path {
    extensions: TABLE_FILES,
};

const DH_LOAD: &[Param] = &[
    setting(
        "collar",
        FILES,
        "/dh/collar",
        L::new("Collar", "Collar", "Collar"),
        "Collar table: hole id, X, Y, Z, length (azimuth/dip optional).",
    ),
    setting(
        "survey",
        FILES,
        "/dh/survey",
        L::new("Survey", "Survey", "Survey"),
        "Survey table: hole id, depth, azimuth, dip (negative down). Empty = straight holes from the collar.",
    ),
    setting(
        "assays",
        FILES,
        "/dh/assays",
        L::new("Leyes (opcional)", "Assays (optional)", "Teores (opcional)"),
        "Assay table: hole id, from, to + numeric attributes. Empty = none.",
    ),
    setting(
        "geo",
        FILES,
        "/dh/geo",
        L::new("Geología (opcional)", "Geology (optional)", "Geologia (opcional)"),
        "Geology table: hole id, from, to + text attributes. Empty = none.",
    ),
];

const DH_DRAW: &[Param] = &[
    setting(
        "draw_trace",
        Kind::Bool,
        "/dh/draw_trace",
        L::new("Traza", "Trace", "Traço"),
        "Draw the hole traces.",
    ),
    setting(
        "draw_collar",
        Kind::Bool,
        "/dh/draw_collar",
        L::new("Etiqueta de collar", "Collar label", "Rótulo do collar"),
        "Draw collar labels.",
    ),
    setting(
        "draw_assays",
        Kind::Bool,
        "/dh/draw_assays",
        L::new(
            "Traza coloreada por ley",
            "Trace coloured by grade",
            "Traço colorido por teor",
        ),
        "Draw assay intervals coloured by assay_attr.",
    ),
    setting(
        "draw_geo",
        Kind::Bool,
        "/dh/draw_geo",
        L::new(
            "Traza coloreada por geología",
            "Trace coloured by geology",
            "Traço colorido por geologia",
        ),
        "Draw geology intervals coloured by geo_attr.",
    ),
    setting(
        "draw_labels",
        Kind::Bool,
        "/dh/draw_labels",
        L::new("Etiquetas de intervalos", "Interval labels", "Rótulos de intervalos"),
        "Label the intervals with their value.",
    ),
    setting(
        "label_height",
        Kind::num(Some(0.01), None, 0.5),
        "/dh/label_height",
        L::new("Altura de texto", "Text height", "Altura do texto"),
        "Text height of the labels (drawing units).",
    ),
    setting(
        "assay_attr",
        Kind::Name {
            source: "assay attributes",
        },
        "/dh/assay_attr",
        L::new("Atributo de ley", "Grade attribute", "Atributo de teor"),
        "Assay attribute (column) that colours the assay intervals.",
    ),
    setting(
        "scheme",
        Kind::Choice {
            options: &[
                opt("blue_red", L::new("Azul→Rojo", "Blue→Red", "Azul→Vermelho")),
                opt("green_red", L::new("Verde→Rojo", "Green→Red", "Verde→Vermelho")),
                opt("rainbow", L::new("Arcoíris", "Rainbow", "Arco-íris")),
                opt("custom", L::new("Personalizado", "Custom", "Personalizado")),
            ],
        },
        "/dh/scheme",
        L::new("Esquema", "Scheme", "Esquema"),
        "Colour scale of the assay intervals; custom uses custom_ranges.",
    ),
    setting(
        "scope",
        Kind::Choice {
            options: &[
                opt(
                    "global",
                    L::new(
                        "Global (todo el proyecto)",
                        "Global (whole project)",
                        "Global (todo o projeto)",
                    ),
                ),
                opt("per_hole", L::new("Por sondaje", "Per hole", "Por furo")),
            ],
        },
        "/dh/scope",
        L::new("Escala", "Scale", "Escala"),
        "Range of the colour scale: whole project or each hole.",
    ),
    setting(
        "custom_ranges",
        Kind::Table {
            columns: &[
                L::new("Mín", "Min", "Mín"),
                L::new("Máx", "Max", "Máx"),
                L::new("Color", "Colour", "Cor"),
            ],
            codec: "ranges",
        },
        "/dh/custom_ranges",
        L::new("Rangos personalizados", "Custom ranges", "Faixas personalizadas"),
        "Colour ranges for the custom scheme: rows of [min, max, colour name or ACI number].",
    ),
    setting(
        "geo_attr",
        Kind::Name {
            source: "geology attributes",
        },
        "/dh/geo_attr",
        L::new("Atributo geológico", "Geology attribute", "Atributo geológico"),
        "Geology attribute (column) that colours the geology intervals.",
    ),
    setting(
        "geo_colors",
        Kind::Table {
            columns: &[L::new("Valor", "Value", "Valor"), L::new("Color", "Colour", "Cor")],
            codec: "geo_colors",
        },
        "/dh/geo_colors",
        L::new("Colores por valor", "Colours by value", "Cores por valor"),
        "Colour per geology value: rows of [value, colour name or ACI number].",
    ),
    setting(
        "origin_enabled",
        Kind::Bool,
        "/origin/enabled",
        L::new(
            "Dibujar respecto a origen local",
            "Draw relative to a local origin",
            "Desenhar relativo a uma origem local",
        ),
        "Draw relative to a local origin (keeps precision with large UTM coordinates).",
    ),
    setting(
        "origin_x",
        Kind::num(None, None, 1000.0),
        "/origin/x",
        L::new("X", "X", "X"),
        "Local origin X (0,0,0 = automatic: first collar rounded to 1000 m).",
    ),
    setting(
        "origin_y",
        Kind::num(None, None, 1000.0),
        "/origin/y",
        L::new("Y", "Y", "Y"),
        "Local origin Y.",
    ),
    setting(
        "origin_z",
        Kind::num(None, None, 100.0),
        "/origin/z",
        L::new("Z", "Z", "Z"),
        "Local origin Z.",
    ),
];

const DH_CLEAR: &[Param] = &[];

const DH_GEOCOLORS: &[Param] = &[setting(
    "geo_attr",
    Kind::Name {
        source: "geology attributes",
    },
    "/dh/geo_attr",
    L::new("Atributo geológico", "Geology attribute", "Atributo geológico"),
    "Geology attribute whose values get a colour.",
)];

pub static OPS: &[OpDef] = &[
OpDef { name: "dh_load", command: "MIN_DH_LOAD", aliases: &[],
            label: L::new("Cargar y validar", "Load and validate", "Carregar e validar"), group: Group::Drillholes, kind: OpKind::Action,
            summary: "Read the drillhole tables (collar required; survey, assays, geology optional; CSV/TXT/TSV/XLSX/XLS/ODS), map their columns (a dialog asks once for columns it cannot recognise; remembered), validate them (rules R01-R07) and keep them for the drawing, composites and geotechnics. Does not draw: run dh_draw.", params: DH_LOAD,
            positional: &["collar", "survey", "assays", "geo"], result: "`holes`, `surveys`, `assays`, `geo` (record counts after validation), `assay_attributes`, `geo_attributes`, `errors` and `warnings` (validation messages), `origin` (local drawing origin, `[x,y,z]`)." },
OpDef { name: "dh_draw", command: "MIN_DH_DRAW", aliases: &[],
            label: L::new("Dibujar", "Draw", "Desenhar"), group: Group::Drillholes, kind: OpKind::Action,
            summary: "Draw the loaded holes: 3D traces, collars, assay and geology intervals coloured by the chosen attribute, labels. Replaces the previous drawing of the same holes. Needs dh_load.", params: DH_DRAW,
            positional: &[], result: "`holes` (drawn), `entities` (created), `removed` (previous entities replaced)." },
OpDef { name: "dh_clear", command: "MIN_DH_CLEAR", aliases: &[],
            label: L::new("Borrar dibujo", "Clear drawing", "Apagar desenho"), group: Group::Drillholes, kind: OpKind::Action,
            summary: "Remove every drillhole entity created by the plugin.", params: DH_CLEAR,
            positional: &[], result: "`removed`." },
OpDef { name: "dh_geocolors", command: "MIN_DH_GEOCOLORS", aliases: &[],
            label: L::new("Generar tabla de colores", "Build colour table", "Gerar tabela de cores"), group: Group::Drillholes, kind: OpKind::Action,
            summary: "Fill the geology colour table (geo_colors) with the distinct values of geo_attr and default colours. Needs dh_load.", params: DH_GEOCOLORS,
            positional: &[], result: "`values` (number of geology values), `colors` (value -> colour)." },
];

pub static PANELS: &[PanelSpec] = &[PanelSpec {
    id: "min_drillholes",
    title: L::new("Sondajes", "Drillholes", "Furos de sondagem"),
    command: "MIN_DH_PANEL",
    items: &[
        Item::Section {
            id: "dh_files",
            title: L::new("Archivos", "Files", "Arquivos"),
            items: &[
                Item::Param("dh_load", "collar"),
                Item::Param("dh_load", "survey"),
                Item::Param("dh_load", "assays"),
                Item::Param("dh_load", "geo"),
            ],
            collapsed: false,
        },
        Item::Button("dh_load"),
        Item::Status("dh_status"),
        Item::Section {
            id: "dh_drawing",
            title: L::new("Dibujo", "Drawing", "Desenho"),
            items: &[
                Item::Param("dh_draw", "draw_trace"),
                Item::Param("dh_draw", "draw_collar"),
                Item::Param("dh_draw", "draw_assays"),
                Item::Param("dh_draw", "draw_geo"),
                Item::Param("dh_draw", "draw_labels"),
                Item::Param("dh_draw", "label_height"),
            ],
            collapsed: false,
        },
        Item::Section {
            id: "dh_colors",
            title: L::new("Colores", "Colours", "Cores"),
            items: &[
                Item::Param("dh_draw", "assay_attr"),
                Item::Param("dh_draw", "scheme"),
                Item::Param("dh_draw", "scope"),
                Item::Param("dh_draw", "custom_ranges"),
                Item::Param("dh_draw", "geo_attr"),
                Item::Button("dh_geocolors"),
                Item::Param("dh_draw", "geo_colors"),
            ],
            collapsed: false,
        },
        Item::Section {
            id: "or_fields",
            title: L::new("Origen local", "Local origin", "Origem local"),
            items: &[
                Item::Param("dh_draw", "origin_enabled"),
                Item::Param("dh_draw", "origin_x"),
                Item::Param("dh_draw", "origin_y"),
                Item::Param("dh_draw", "origin_z"),
                Item::Info(L::new(
                    "Para coordenadas UTM grandes. 0,0,0 = automático (primer collar redondeado a 1000 m).",
                    "For large UTM coordinates. 0,0,0 = automatic (first collar rounded to 1000 m).",
                    "Para coordenadas UTM grandes. 0,0,0 = automático (primeiro collar arredondado a 1000 m).",
                )),
            ],
            collapsed: false,
        },
        Item::Row(&[Item::Button("dh_draw"), Item::Button("dh_clear")]),
    ],
}];
