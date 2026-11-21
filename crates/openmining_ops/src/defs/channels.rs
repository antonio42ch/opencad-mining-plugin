//! Channel samples of underground workings converted into drillhole data.

use crate::{input, opt, setting, Group, Item, Kind, OpDef, OpKind, Opt, PanelSpec, Param, L, TABLE_FILES};

const FILES: Kind = Kind::Path {
    extensions: TABLE_FILES,
};

const ANCHORS: &[Opt] = &[
    opt(
        "centre",
        L::new(
            "Centrado en la posición",
            "Centred on the position",
            "Centrado na posição",
        ),
    ),
    opt(
        "start",
        L::new("Empieza en la posición", "Starts at the position", "Começa na posição"),
    ),
];

const CONVERT: &[Param] = &[
    setting("points", FILES, "/ch/points", L::new("Puntos topográficos", "Topographic points", "Pontos topográficos"),
        "Topographic points table: id, east (X), north (Y), elevation (Z)."),
    setting("sheet", FILES, "/ch/sheet", L::new("Hoja de muestreo de canales", "Channel sampling sheet", "Planilha de amostragem de canais"),
        "One row per sample: channel, topographic point, distance from it, next point (or azimuth of the working), sample width (or from/to), vein thickness (potencia), grades; optional: position, height, sample type, working, sample id, channel azimuth / inclination."),
    setting("prefix", Kind::Text, "/ch/prefix", L::new("Prefijo de los canales", "Channel id prefix", "Prefixo dos canais"), "Prefix added to the channel ids to make the hole ids (CH-)."),
    setting("anchor", Kind::Choice { options: ANCHORS }, "/ch/anchor", L::new("Ubicación del canal", "Channel placement", "Posição do canal"), "The channel is centred on the located position, or starts there."),
    setting("height", Kind::num(None, None, 0.5), "/ch/height", L::new("Altura sobre el punto (m)", "Height above the point (m)", "Altura sobre o ponto (m)"), "Added to the point elevation when the sheet has no height column."),
    setting("azimuth_offset", Kind::num(Some(-360.0), Some(360.0), 5.0), "/ch/azimuth_offset", L::new("Ángulo respecto de la labor (°)", "Angle to the working (°)", "Ângulo em relação à galeria (°)"),
        "Default channel azimuth = working direction + this (90 = perpendicular, across the vein when the working follows it)."),
    setting("inclination", Kind::num(Some(-90.0), Some(90.0), 5.0), "/ch/inclination", L::new("Inclinación por defecto (°)", "Default inclination (°)", "Inclinação padrão (°)"), "Default channel inclination, positive up (0 = horizontal)."),
    setting("auto", Kind::Bool, "/ch/auto", L::new("Reconvertir al cargar sondajes", "Convert again when loading drillholes", "Converter de novo ao carregar furos"),
        "Convert the channels again (and append them) every time the drillholes are loaded."),
    setting("write_files", Kind::Bool, "/ch/write_files", L::new("Guardar tablas tipo sondaje", "Save drillhole-format tables", "Salvar tabelas tipo furo"),
        "Write the converted collar / survey / assays / geology CSV next to the drawing."),
];

const ORIENT: &[Param] = &[
    setting(
        "channel",
        Kind::Text,
        "/ch/target",
        L::new("Canal", "Channel", "Canal"),
        "Channel id (as in the sheet, without prefix) whose orientation is set.",
    ),
    setting(
        "orientations",
        Kind::Table {
            columns: &[
                L::new("Canal", "Channel", "Canal"),
                L::new("Azimut", "Azimuth", "Azimute"),
                L::new("Inclinación", "Inclination", "Inclinação"),
            ],
            codec: "ch_orient",
        },
        "/ch/orient",
        L::new("Orientaciones definidas", "Orientations set", "Orientações definidas"),
        "Channel orientations that replace the default: rows of [channel, azimuth, inclination].",
    ),
    input(
        "p1",
        Kind::Point,
        false,
        L::new("Punto 1", "Point 1", "Ponto 1"),
        "Start of the channel direction (drawing coordinates). With p1 and p2 no pick is needed.",
    ),
    input(
        "p2",
        Kind::Point,
        false,
        L::new("Punto 2", "Point 2", "Ponto 2"),
        "End of the channel direction.",
    ),
];

pub static OPS: &[OpDef] = &[
    OpDef {
        name: "ch_convert",
        command: "MIN_CH_CONVERT",
        aliases: &[],
        label: L::new("Convertir canales", "Convert channels", "Converter canais"),
        group: Group::Drillholes,
        kind: OpKind::Action,
        summary: "Convert a channel sampling sheet into drillhole data: each channel is located from a topographic point and a distance along the working (towards the next point or an azimuth), horizontal and perpendicular to the working by default, and becomes a short hole (id prefix CH-) with its samples, thickness (potencia), grades and sample type. The channels are appended to the loaded drillholes (replacing earlier channels), so dh_draw, rs_composite, gt_* and the vein model use them.",
        params: CONVERT,
        positional: &["points", "sheet"],
        result: "`channels`, `samples`, `issues` (list of `{channel, row, message}` for skipped channels), `grade_columns`, `text_columns`, `holes` (drillholes + channels loaded), `files` (written CSV or null).",
    },
    OpDef {
        name: "ch_orient",
        command: "MIN_CH_ORIENT",
        aliases: &[],
        label: L::new("Orientar canal en pantalla", "Orient channel on screen", "Orientar canal na tela"),
        group: Group::Drillholes,
        kind: OpKind::Action,
        summary: "Set the orientation of one channel from two points (picked on screen, or given as p1 and p2): azimuth and inclination of the line p1 -> p2. It is kept in the orientations table and the channels are converted again.",
        params: ORIENT,
        positional: &["channel"],
        result: "`channel`, `azimuth`, `inclination`, and the ch_convert result under `converted`.",
    },
];

pub static PANELS: &[PanelSpec] = &[PanelSpec {
    id: "min_channels",
    title: L::new("Canales de muestreo", "Channel samples", "Canais de amostragem"),
    command: "MIN_CH_PANEL",
    items: &[
        Item::Section {
            id: "ch_f_files",
            title: L::new("Archivos", "Files", "Arquivos"),
            items: &[Item::Param("ch_convert", "points"), Item::Param("ch_convert", "sheet")],
            collapsed: false,
        },
        Item::Section {
            id: "ch_f_opts",
            title: L::new(
                "Ubicación y orientación",
                "Placement and orientation",
                "Posição e orientação",
            ),
            items: &[
                Item::Param("ch_convert", "anchor"),
                Item::Param("ch_convert", "height"),
                Item::Param("ch_convert", "azimuth_offset"),
                Item::Param("ch_convert", "inclination"),
                Item::Param("ch_convert", "prefix"),
                Item::Param("ch_convert", "auto"),
                Item::Param("ch_convert", "write_files"),
            ],
            collapsed: false,
        },
        Item::Button("ch_convert"),
        Item::Status("ch_status"),
        Item::Section {
            id: "ch_f_orient",
            title: L::new(
                "Orientación por canal",
                "Orientation per channel",
                "Orientação por canal",
            ),
            items: &[
                Item::Param("ch_orient", "channel"),
                Item::Button("ch_orient"),
                Item::Param("ch_orient", "orientations"),
            ],
            collapsed: true,
        },
        Item::Info(L::new(
            "Los canales se agregan a los sondajes cargados: dibújelos con Sondajes → Dibujar.",
            "The channels are appended to the loaded drillholes: draw them with Drillholes → Draw.",
            "Os canais são adicionados aos furos carregados: desenhe-os com Furos → Desenhar.",
        )),
    ],
}];
