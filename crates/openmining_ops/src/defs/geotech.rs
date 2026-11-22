//! Geotechnics (phase 1): tables, RMR89 / Q / GSI, Hoek-Brown, block model.

use crate::{input, opt, setting, Group, Item, Kind, OpDef, OpKind, Opt, PanelSpec, Param, L, TABLE_FILES};

const FILES: Kind = Kind::Path {
    extensions: TABLE_FILES,
};

const LOAD: &[Param] = &[
    setting("logging", FILES, "/gt/logging", L::new("Logueo de sondajes", "Drillhole logging", "Descrição de furos"),
        "Drillhole logging table by interval (hole, from, to + RQD, UCS, spacing, condition, water, Jn, Jr, Ja, Jw, SRF, ratings...). Located on the holes loaded with dh_load. Empty string = none."),
    setting("mapping", FILES, "/gt/mapping", L::new("Mapeo de frentes / celdas", "Face / cell mapping", "Mapeamento de frentes / células"),
        "Face or cell mapping table: X/Y[/Z] coordinates, or chainage (PK) along a working axis, + the same rating parameters. Empty string = none."),
    setting("lab", FILES, "/gt/lab", L::new("Ensayos de laboratorio", "Laboratory tests", "Ensaios de laboratório"),
        "Laboratory table by lithology: UCS, Is50, mi, Ei, Poisson, density. Supplies UCS when a record lacks it. Empty string = none."),
    setting("joints", FILES, "/gt/joints", L::new("Discontinuidades orientadas", "Oriented discontinuities", "Descontinuidades orientadas"),
        "Oriented discontinuities (dip, dip direction...). Loaded and counted; analysed in a later phase. Empty string = none."),
];

const RMR_VERSIONS: &[Opt] = &[opt("rmr89", L::same("RMR89 (Bieniawski 1989)"))];
const CORRELATIONS: &[Opt] = &[
    opt(
        "bieniawski",
        L::new(
            "Bieniawski: RMR = 9 ln Q + 44",
            "Bieniawski: RMR = 9 ln Q + 44",
            "Bieniawski: RMR = 9 ln Q + 44",
        ),
    ),
    opt(
        "rutledge_preston",
        L::new(
            "Rutledge-Preston: RMR = 5,9 ln Q + 43",
            "Rutledge-Preston: RMR = 5.9 ln Q + 43",
            "Rutledge-Preston: RMR = 5,9 ln Q + 43",
        ),
    ),
    opt("barton", L::same("Barton: RMR = 15 log Q + 50")),
];
const COLOR_BY: &[Opt] = &[
    opt(
        "rmr_class",
        L::new("Clase RMR (I–V)", "RMR class (I–V)", "Classe RMR (I–V)"),
    ),
    opt("q_class", L::new("Clase Q", "Q class", "Classe Q")),
    opt(
        "gsi",
        L::new("GSI (tramos de 20)", "GSI (20-point bins)", "GSI (faixas de 20)"),
    ),
];

const CLASSIFY: &[Param] = &[
    setting("rmr_version", Kind::Choice { options: RMR_VERSIONS }, "/gt/rmr_version", L::new("Versión de RMR", "RMR version", "Versão do RMR"),
        "RMR system. Only RMR89 (official tables) for now."),
    setting("correlation", Kind::Choice { options: CORRELATIONS }, "/gt/correlation", L::new("Correlación RMR ↔ Q", "RMR ↔ Q correlation", "Correlação RMR ↔ Q"),
        "Correlation used by cross_fill."),
    setting("cross_fill", Kind::Bool, "/gt/cross_fill", L::new("Completar RMR o Q con la correlación cuando falte uno", "Fill RMR or Q with the correlation when one is missing", "Completar RMR ou Q com a correlação quando faltar um"),
        "When a record has only RMR or only Q, derive the other with the correlation (reported as derived). Off = never invent values."),
    setting("rqd_from_ff", Kind::Bool, "/gt/rqd_from_ff", L::new("Estimar RQD desde fracturas/m (Priest-Hudson)", "Estimate RQD from fractures/m (Priest-Hudson)", "Estimar RQD a partir de fraturas/m (Priest-Hudson)"),
        "Estimate RQD from the fracture frequency (Priest & Hudson 1976) when RQD is not logged."),
    setting("color_by", Kind::Choice { options: COLOR_BY }, "/gt/color_by", L::new("Colorear por", "Colour by", "Colorir por"),
        "Colour of the drawn records."),
    setting("labels", Kind::Bool, "/gt/labels", L::new("Etiquetas en el mapeo", "Labels on mapping", "Rótulos no mapeamento"),
        "Draw an 'RMR · Q · GSI' label at each mapped record."),
    setting("label_height", Kind::num(Some(0.01), None, 0.5), "/gt/label_height", L::new("Altura de etiqueta", "Label height", "Altura do rótulo"),
        "Text height of the labels (drawing units)."),
    input("axes", Kind::Handles, false, L::new("Ejes", "Axes", "Eixos"),
        "Axis polylines of the workings, for mapping records located by chainage. With several axes the table's Axis column is matched to each polyline's layer. Without it a pick is started (status waiting_input)."),
];

const APPLICATIONS: &[Opt] = &[
    opt("tunnel", L::new("Túnel / labor", "Tunnel / drift", "Túnel / galeria")),
    opt("slope", L::new("Talud", "Slope", "Talude")),
];

const HB: &[Param] = &[
    setting(
        "sigci",
        Kind::num(Some(0.1), None, 10.0),
        "/gt/hb_sigci",
        L::new(
            "σci roca intacta (MPa)",
            "σci intact rock (MPa)",
            "σci rocha intacta (MPa)",
        ),
        "Uniaxial compressive strength of the intact rock (MPa).",
    ),
    setting(
        "gsi",
        Kind::num(Some(0.0), Some(100.0), 5.0),
        "/gt/hb_gsi",
        L::same("GSI"),
        "Geological Strength Index of the rock mass.",
    ),
    setting(
        "mi",
        Kind::num(Some(0.1), None, 1.0),
        "/gt/hb_mi",
        L::same("mi"),
        "Hoek-Brown constant of the intact rock.",
    ),
    setting(
        "d",
        Kind::num(Some(0.0), Some(1.0), 0.1),
        "/gt/hb_d",
        L::new("Perturbación D (0–1)", "Disturbance D (0–1)", "Perturbação D (0–1)"),
        "Disturbance factor (blast damage, stress relaxation).",
    ),
    setting(
        "ei",
        Kind::num(Some(0.0), None, 1000.0),
        "/gt/hb_ei",
        L::new(
            "Ei roca intacta (MPa, 0 = simplificado)",
            "Ei intact rock (MPa, 0 = simplified)",
            "Ei rocha intacta (MPa, 0 = simplificado)",
        ),
        "Intact rock modulus (MPa) for the generalised Hoek-Diederichs modulus; 0 uses the simplified equation.",
    ),
    setting(
        "application",
        Kind::Choice { options: APPLICATIONS },
        "/gt/hb_app",
        L::new("Aplicación", "Application", "Aplicação"),
        "Range of the Mohr-Coulomb fit: tunnels or slopes (Hoek et al. 2002).",
    ),
    setting(
        "gamma",
        Kind::num(Some(1.0), None, 1.0),
        "/gt/hb_gamma",
        L::new(
            "Peso unitario (kN/m³)",
            "Unit weight (kN/m³)",
            "Peso específico (kN/m³)",
        ),
        "Unit weight of the rock mass (kN/m3).",
    ),
    setting(
        "depth",
        Kind::num(Some(1.0), None, 50.0),
        "/gt/hb_depth",
        L::new(
            "Profundidad / altura (m)",
            "Depth / height (m)",
            "Profundidade / altura (m)",
        ),
        "Depth of the tunnel or height of the slope (m).",
    ),
];

const HB_DATA: &[Param] = &[setting("lithology", Kind::Text, "/gt/hb_lith", L::new("Litología (laboratorio)", "Lithology (laboratory)", "Litologia (laboratório)"),
    "Lithology whose classified records give the median GSI and whose laboratory row gives σci, mi and Ei. Empty = all records, no laboratory.")];

const METHODS: &[Opt] = &[
    opt(
        "idw",
        L::new("Inverso de la distancia", "Inverse distance", "Inverso da distância"),
    ),
    opt(
        "nn",
        L::new("Vecino más cercano", "Nearest neighbour", "Vizinho mais próximo"),
    ),
];

const ESTIMATE: &[Param] = &[
    setting(
        "method",
        Kind::Choice { options: METHODS },
        "/gt/md_method",
        L::new("Método", "Method", "Método"),
        "Interpolation method.",
    ),
    setting(
        "power",
        Kind::num(Some(0.1), None, 0.5),
        "/gt/md_power",
        L::new("Potencia IDW", "IDW power", "Potência IDW"),
        "Inverse distance power.",
    ),
    setting(
        "radius1",
        Kind::num(Some(0.1), None, 10.0),
        "/gt/md_radii/0",
        L::new("Radio 1", "Radius 1", "Raio 1"),
        "Search radius along the azimuth (m).",
    ),
    setting(
        "radius2",
        Kind::num(Some(0.1), None, 10.0),
        "/gt/md_radii/1",
        L::new("Radio 2", "Radius 2", "Raio 2"),
        "Horizontal search radius across the azimuth (m).",
    ),
    setting(
        "radius3",
        Kind::num(Some(0.1), None, 10.0),
        "/gt/md_radii/2",
        L::new("Radio 3", "Radius 3", "Raio 3"),
        "Vertical search radius (m).",
    ),
    setting(
        "azimuth",
        Kind::num(None, None, 5.0),
        "/gt/md_az",
        L::new("Azimut del radio 1", "Azimuth of radius 1", "Azimute do raio 1"),
        "Azimuth of radius 1 (degrees from north).",
    ),
    setting(
        "min_samples",
        Kind::num(Some(1.0), None, 1.0),
        "/gt/md_min",
        L::new("Mínimo de muestras", "Minimum samples", "Mínimo de amostras"),
        "Minimum records to estimate a block.",
    ),
    setting(
        "max_samples",
        Kind::num(Some(1.0), None, 1.0),
        "/gt/md_max",
        L::new("Máximo de muestras", "Maximum samples", "Máximo de amostras"),
        "Maximum records used per block.",
    ),
    setting(
        "max_per_hole",
        Kind::num(Some(0.0), None, 1.0),
        "/gt/md_hole",
        L::new(
            "Máximo por sondaje (0 = sin límite)",
            "Maximum per hole (0 = no limit)",
            "Máximo por furo (0 = sem limite)",
        ),
        "Maximum records per drillhole (0 = no limit).",
    ),
    setting(
        "honour_domains",
        Kind::Bool,
        "/gt/md_domains",
        L::new(
            "Respetar dominios de Recursos (por litología)",
            "Honour resource domains (by lithology)",
            "Respeitar domínios de Recursos (por litologia)",
        ),
        "Each record only informs the resource domain whose values list its lithology.",
    ),
];

const ATTRS: &[Opt] = &[
    opt("rmr", L::same("RMR")),
    opt("gsi", L::same("GSI")),
    opt("logq", L::same("log Q")),
    opt(
        "class",
        L::new("Clase de macizo", "Rock mass class", "Classe do maciço"),
    ),
    opt(
        "dist",
        L::new(
            "Distancia media a los registros (confianza)",
            "Mean distance to the records (confidence)",
            "Distância média aos registros (confiança)",
        ),
    ),
    opt(
        "n",
        L::new(
            "N° de registros usados (confianza)",
            "Number of records used (confidence)",
            "N.º de registros usados (confiança)",
        ),
    ),
];

const SHOW: &[Opt] = &[
    opt("all", L::new("Todos los bloques", "All blocks", "Todos os blocos")),
    opt(
        "below",
        L::new(
            "Solo valores menores que el umbral",
            "Only values below the threshold",
            "Somente valores abaixo do limiar",
        ),
    ),
    opt(
        "above",
        L::new(
            "Solo valores mayores que el umbral",
            "Only values above the threshold",
            "Somente valores acima do limiar",
        ),
    ),
];

const CUT_SIDES: &[Opt] = &[
    opt(
        "front",
        L::new(
            "El lado hacia el que apunta el plano",
            "The side the plane faces",
            "O lado para onde o plano aponta",
        ),
    ),
    opt("back", L::new("El lado opuesto", "The opposite side", "O lado oposto")),
];

const VIEW_MODES: &[Opt] = &[
    opt("levels", L::new("Niveles (planta)", "Levels (plan)", "Níveis (planta)")),
    opt(
        "two_points",
        L::new("Secciones por 2 puntos", "Sections by 2 points", "Seções por 2 pontos"),
    ),
    opt(
        "axis",
        L::new(
            "Secciones perpendiculares a un eje",
            "Sections perpendicular to an axis",
            "Seções perpendiculares a um eixo",
        ),
    ),
    opt(
        "cubes",
        L::new(
            "Cubos (un cubo por bloque)",
            "Cubes (one cube per block)",
            "Cubos (um cubo por bloco)",
        ),
    ),
];

const VIEW: &[Param] = &[
    setting("mode", Kind::Choice { options: VIEW_MODES }, "/gt/view_mode", L::new("Tipo de vista", "View type", "Tipo de vista"),
        "levels = horizontal plans; two_points / axis = vertical sections; cubes = every block with a value as a cube (coloured meshes, one per colour)."),
    setting("attribute", Kind::Choice { options: ATTRS }, "/gt/md_attr", L::new("Atributo", "Attribute", "Atributo"),
        "Geotechnical column to colour the block model view with (class uses the class colours; others show poor rock in red)."),
    setting("z_from", Kind::num(None, None, 12.0), "/gt/view_z_from", L::new("Nivel inicial", "Lowest level", "Nível inicial"), "Lowest level (mode levels; from = to = every block level)."),
    setting("z_to", Kind::num(None, None, 12.0), "/gt/view_z_to", L::new("Nivel final", "Highest level", "Nível final"), "Highest level (mode levels)."),
    setting("z_step", Kind::num(Some(0.1), None, 1.0), "/gt/view_z_step", L::new("Paso entre niveles", "Level spacing", "Passo entre níveis"), "Level spacing (mode levels, m)."),
    setting("spacing", Kind::num(Some(0.1), None, 5.0), "/gt/view_spacing", L::new("Separación de secciones", "Section spacing", "Espaçamento das seções"), "Spacing of parallel sections (mode two_points, m)."),
    setting("count", Kind::num(Some(1.0), None, 1.0), "/gt/view_count", L::new("Número de secciones", "Number of sections", "Número de seções"), "Number of parallel sections (mode two_points)."),
    setting("interval", Kind::num(Some(0.1), None, 5.0), "/gt/view_interval", L::new("Intervalo a lo largo del eje", "Interval along the axis", "Intervalo ao longo do eixo"), "Section spacing along the axis (mode axis, m)."),
    setting("limit", Kind::num(Some(1.0), None, 1000.0), "/gt/view_limit", L::new("Máximo de polígonos / cubos", "Maximum polygons / cubes", "Máximo de polígonos / cubos"),
        "Maximum polygons or cubes drawn; above it cubes are an even sample of the blocks."),
    setting("show", Kind::Choice { options: SHOW }, "/gt/view_filter", L::new("Mostrar", "Show", "Mostrar"),
        "Show every block, or only those whose attribute value is below / above `threshold` (e.g. attribute=class show=above threshold=3 shows classes IV and V; attribute=rmr show=below threshold=40 shows poor rock)."),
    setting("threshold", Kind::num(None, None, 1.0), "/gt/view_threshold", L::new("Umbral (unidades del atributo)", "Threshold (attribute units)", "Limiar (unidades do atributo)"),
        "Threshold of `show`, in the units of the attribute (RMR, GSI, log Q, class number, metres, records)."),
    setting("cut", Kind::Bool, "/gt/view_cut", L::new("Cortar con un plano", "Cut with a plane", "Cortar com um plano"),
        "Keep only the blocks on one side of a plane, to see inside the model (a vertical plane to see a section, a horizontal one to remove the top)."),
    setting("cut_e", Kind::num(None, None, 10.0), "/gt/view_cut_e", L::new("Plano: punto E", "Plane: point E", "Plano: ponto E"), "East of a point of the cutting plane (world)."),
    setting("cut_n", Kind::num(None, None, 10.0), "/gt/view_cut_n", L::new("Plano: punto N", "Plane: point N", "Plano: ponto N"), "North of a point of the cutting plane."),
    setting("cut_z", Kind::num(None, None, 10.0), "/gt/view_cut_z", L::new("Plano: punto Z", "Plane: point Z", "Plano: ponto Z"), "Elevation of a point of the cutting plane."),
    setting("cut_dip", Kind::num(Some(0.0), Some(90.0), 5.0), "/gt/view_cut_dip", L::new("Plano: buzamiento (90 = vertical)", "Plane: dip (90 = vertical)", "Plano: mergulho (90 = vertical)"), "Dip of the cutting plane (0 = horizontal, 90 = vertical)."),
    setting("cut_dipdir", Kind::num(Some(0.0), Some(360.0), 5.0), "/gt/view_cut_dipdir", L::new("Plano: dirección de buzamiento", "Plane: dip direction", "Plano: direção de mergulho"),
        "Dip direction of the cutting plane (azimuth); for a vertical plane it is the direction its normal points to."),
    setting("cut_side", Kind::Choice { options: CUT_SIDES }, "/gt/view_cut_side", L::new("Lado que se conserva", "Side kept", "Lado mantido"),
        "front = the side the plane faces (towards its dip direction, or up for a horizontal plane); back = the other side."),
    input("axis", Kind::Handle, false, L::new("Eje", "Axis", "Eixo"), "Axis polyline (mode axis). Without it a pick is started."),
    input("p1", Kind::Point, false, L::new("Punto 1", "Point 1", "Ponto 1"), "First point of the section line (mode two_points)."),
    input("p2", Kind::Point, false, L::new("Punto 2", "Point 2", "Ponto 2"), "Second point of the section line."),
];

pub static OPS: &[OpDef] = &[
    OpDef {
        name: "gt_load",
        command: "MIN_GT_LOAD",
        aliases: &[],
        label: L::new("Cargar", "Load", "Carregar"),
        group: Group::Geotechnics,
        kind: OpKind::Action,
        summary: "Read the geotechnical tables (logging, face mapping, laboratory, discontinuities; CSV/TXT/TSV/XLSX/XLS/ODS). Columns are recognised by name in es/en/pt; a missing required column opens a column chooser once (remembered). Replaces the tables loaded before.",
        params: LOAD,
        positional: &["logging", "mapping", "lab", "joints"],
        result: "`tables`: per table (`logging`, `mapping`, `lab`, `joints`) `records`, `columns` (recognised fields), `unused` (headers); `lab_lithologies`; `errors` (messages of tables that could not be read).",
    },
    OpDef {
        name: "gt_classify",
        command: "MIN_GT_CLASSIFY",
        aliases: &[],
        label: L::new("Clasificar y dibujar", "Classify and draw", "Classificar e desenhar"),
        group: Group::Geotechnics,
        kind: OpKind::Action,
        summary: "Classify every logging and mapping record (RMR89, Q, Q', GSI, class I-V), locate it (drillholes from dh_load, mapping by coordinates or chainage) and draw it coloured, replacing the previous drawing. Missing parameters are reported, never invented.",
        params: CLASSIFY,
        positional: &[],
        result: "`records`, `with_rmr`, `with_q`, `with_gsi`, `unclassified`, `classes` ([I..V] counts), `missing` (parameter -> records), `invalid` (list of `{table,row,field,value}`), `derived` (kind -> records), `unlocated` (count) and `unlocated_examples`, `drawn`.",
    },
    OpDef {
        name: "gt_clear",
        command: "MIN_GT_CLEAR",
        aliases: &[],
        label: L::new("Borrar dibujo", "Clear drawing", "Apagar desenho"),
        group: Group::Geotechnics,
        kind: OpKind::Action,
        summary: "Remove the drawn geotechnical records.",
        params: &[],
        positional: &[],
        result: "`removed`.",
    },
    OpDef {
        name: "gt_hb",
        command: "MIN_GT_HB",
        aliases: &[],
        label: L::new("Calcular", "Compute", "Calcular"),
        group: Group::Geotechnics,
        kind: OpKind::Action,
        summary: "Generalised Hoek-Brown 2002: mb, s, a, rock mass strengths, equivalent Mohr-Coulomb c' and phi' (tunnel or slope range) and rock mass modulus (Hoek-Diederichs 2006); shows the envelope chart in the panel.",
        params: HB,
        positional: &[],
        result: "`mb`, `s`, `a`, `sigma_c`, `sigma_t`, `sigma_cm`, `erm` (MPa), `sigma3_max`, `c` (MPa), `phi` (degrees).",
    },
    OpDef {
        name: "gt_hb_data",
        command: "MIN_GT_HB_DATA",
        aliases: &[],
        label: L::new("Usar datos clasificados", "Use classified data", "Usar dados classificados"),
        group: Group::Geotechnics,
        kind: OpKind::Action,
        summary: "Set the Hoek-Brown GSI to the median of the classified records (of the lithology, if given) and sigci / mi / Ei from its laboratory row, then compute like gt_hb. Needs gt_classify first.",
        params: HB_DATA,
        positional: &[],
        result: "`gsi`, `records` (used for the median), `lab` (true when the lithology had a laboratory row), `sigci`, `mi`, `ei`, and the gt_hb fields under `hoek_brown`.",
    },
    OpDef {
        name: "gt_estimate",
        command: "MIN_GT_ESTIMATE",
        aliases: &[],
        label: L::new("Estimar geotecnia", "Estimate geotechnics", "Estimar geotecnia"),
        group: Group::Geotechnics,
        kind: OpKind::Action,
        summary: "Interpolate RMR, GSI and log10 Q of the classified records into the block model (the resource model, its .ombm file, or a new one with the resource geometry and one Global domain); writes gt_rmr, gt_gsi, gt_logq, gt_class and the confidence columns gt_dist (mean distance to the records used, m) and gt_n (number of records used). Density and tonnage are not touched. Needs gt_classify first.",
        params: ESTIMATE,
        positional: &[],
        result: "`estimated`, `blocks` (in a domain), `no_domain` (records skipped by honour_domains), `columns`, `model_created`, `saved` (file path or null).",
    },
    OpDef {
        name: "gt_view",
        command: "MIN_GT_VIEW",
        aliases: &[],
        label: L::new("Ver modelo", "View model", "Ver modelo"),
        group: Group::Geotechnics,
        kind: OpKind::Action,
        summary: "Draw the block model coloured by a geotechnical column as level plans, vertical sections (2 points or along an axis) or cubes (mode=cubes: one cube per block with a value, grouped by colour, at most `limit`). Replaces the previous model view.",
        params: VIEW,
        positional: &["attribute"],
        result: "`attribute`, `column`, `mode`, and the view result: `planes`, `polygons`, `limited`, `cubes` (`[drawn, total]` in cubes mode).",
    },
];

const DATA_ITEMS: &[Item] = &[
    Item::Section {
        id: "gt_f_files",
        title: L::new(
            "Archivos (CSV / Excel)",
            "Files (CSV / Excel)",
            "Arquivos (CSV / Excel)",
        ),
        items: &[
            Item::Param("gt_load", "logging"),
            Item::Param("gt_load", "mapping"),
            Item::Param("gt_load", "lab"),
            Item::Param("gt_load", "joints"),
        ],
        collapsed: false,
    },
    Item::Button("gt_load"),
    Item::Section {
        id: "gt_f_opts",
        title: L::new("Clasificación", "Classification", "Classificação"),
        items: &[
            Item::Param("gt_classify", "rmr_version"),
            Item::Info(L::new(
                "RMR14 se agregará cuando se verifiquen sus tablas oficiales.",
                "RMR14 will be added once its official tables are verified.",
                "O RMR14 será adicionado quando suas tabelas oficiais forem verificadas.",
            )),
            Item::Param("gt_classify", "correlation"),
            Item::Param("gt_classify", "cross_fill"),
            Item::Param("gt_classify", "rqd_from_ff"),
        ],
        collapsed: false,
    },
    Item::Section {
        id: "gt_f_draw",
        title: L::new("Dibujo", "Drawing", "Desenho"),
        items: &[
            Item::Param("gt_classify", "color_by"),
            Item::Param("gt_classify", "labels"),
            Item::Param("gt_classify", "label_height"),
        ],
        collapsed: false,
    },
    Item::Row(&[Item::Button("gt_classify"), Item::Button("gt_clear")]),
    Item::Status("gt_status"),
];

const HB_ITEMS: &[Item] = &[
    Item::Section {
        id: "gt_f_hb",
        title: L::new("Macizo rocoso", "Rock mass", "Maciço rochoso"),
        items: &[
            Item::Param("gt_hb", "sigci"),
            Item::Param("gt_hb", "gsi"),
            Item::Param("gt_hb", "mi"),
            Item::Param("gt_hb", "d"),
            Item::Param("gt_hb", "ei"),
            Item::Param("gt_hb", "application"),
            Item::Param("gt_hb", "gamma"),
            Item::Param("gt_hb", "depth"),
            Item::Param("gt_hb_data", "lithology"),
        ],
        collapsed: false,
    },
    Item::Row(&[Item::Button("gt_hb"), Item::Button("gt_hb_data")]),
    Item::Status("gt_hb_out"),
    Item::Chart("gt_hb_chart"),
];

const MODEL_ITEMS: &[Item] = &[
    Item::Info(L::new(
        "Usa el modelo de bloques de Recursos (o crea uno con su geometría). Q se interpola en escala logarítmica. La vista puede ser por niveles, secciones o cubos.",
        "Uses the resource block model (or creates one with its geometry). Q is interpolated in log scale. The view can be level plans, sections or cubes.",
        "Usa o modelo de blocos de Recursos (ou cria um com sua geometria). Q é interpolado em escala logarítmica. A vista pode ser por níveis, seções ou cubos.",
    )),
    Item::Section {
        id: "gt_f_search",
        title: L::new("Búsqueda", "Search", "Busca"),
        items: &[
            Item::Param("gt_estimate", "method"),
            Item::Param("gt_estimate", "power"),
            Item::Param("gt_estimate", "radius1"),
            Item::Param("gt_estimate", "radius2"),
            Item::Param("gt_estimate", "radius3"),
            Item::Param("gt_estimate", "azimuth"),
            Item::Param("gt_estimate", "min_samples"),
            Item::Param("gt_estimate", "max_samples"),
            Item::Param("gt_estimate", "max_per_hole"),
            Item::Param("gt_estimate", "honour_domains"),
        ],
        collapsed: false,
    },
    Item::Button("gt_estimate"),
    Item::Status("gt_md_status"),
    Item::Section {
        id: "gt_f_view",
        title: L::new("Vista", "View", "Vista"),
        items: &[
            Item::Param("gt_view", "mode"),
            Item::Param("gt_view", "attribute"),
            Item::Param("gt_view", "z_from"),
            Item::Param("gt_view", "z_to"),
            Item::Param("gt_view", "z_step"),
            Item::Param("gt_view", "spacing"),
            Item::Param("gt_view", "count"),
            Item::Param("gt_view", "interval"),
            Item::Param("gt_view", "limit"),
            Item::Section {
                id: "gt_f_filter",
                title: L::new("Filtro y corte", "Filter and cut", "Filtro e corte"),
                items: &[
                    Item::Param("gt_view", "show"),
                    Item::Param("gt_view", "threshold"),
                    Item::Param("gt_view", "cut"),
                    Item::Param("gt_view", "cut_e"),
                    Item::Param("gt_view", "cut_n"),
                    Item::Param("gt_view", "cut_z"),
                    Item::Param("gt_view", "cut_dip"),
                    Item::Param("gt_view", "cut_dipdir"),
                    Item::Param("gt_view", "cut_side"),
                ],
                collapsed: true,
            },
            Item::Button("gt_view"),
        ],
        collapsed: false,
    },
];

pub static PANELS: &[PanelSpec] = &[
    PanelSpec {
        id: "min_gt_data",
        title: L::new(
            "Geotecnia: datos y clasificación",
            "Geotechnics: data and classification",
            "Geotecnia: dados e classificação",
        ),
        command: "MIN_GT_DATA_PANEL",
        items: DATA_ITEMS,
    },
    PanelSpec {
        id: "min_gt_hb",
        title: L::new(
            "Geotecnia: Hoek-Brown",
            "Geotechnics: Hoek-Brown",
            "Geotecnia: Hoek-Brown",
        ),
        command: "MIN_GT_HB_PANEL",
        items: HB_ITEMS,
    },
    PanelSpec {
        id: "min_gt_model",
        title: L::new(
            "Geotecnia: modelo de bloques",
            "Geotechnics: block model",
            "Geotecnia: modelo de blocos",
        ),
        command: "MIN_GT_MODEL_PANEL",
        items: MODEL_ITEMS,
    },
];
