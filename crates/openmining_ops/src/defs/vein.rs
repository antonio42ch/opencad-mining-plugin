//! Narrow veins: hanging-wall and footwall surfaces (stage B1).

use crate::{input, opt, setting, Group, Item, Kind, OpDef, OpKind, Opt, PanelSpec, Param, L};

const ORIENT_MODES: &[Opt] = &[
    opt(
        "fit",
        L::new(
            "Ajustado a los contactos",
            "Fitted to the contacts",
            "Ajustado aos contatos",
        ),
    ),
    opt(
        "typed",
        L::new(
            "Buzamiento y dirección escritos",
            "Typed dip and dip direction",
            "Mergulho e direção digitados",
        ),
    ),
];

const METHODS: &[Opt] = &[
    opt(
        "kriging",
        L::new("Krigeado ordinario", "Ordinary kriging", "Krigagem ordinária"),
    ),
    opt(
        "idw",
        L::new("Inverso de la distancia", "Inverse distance", "Inverso da distância"),
    ),
];

const SURFACES: &[Param] = &[
    setting(
        "vein",
        Kind::Text,
        "/vn/name",
        L::new("Veta", "Vein", "Veio"),
        "Name of the vein (its surfaces and model are stored under it).",
    ),
    setting(
        "hw_layer",
        Kind::Text,
        "/vn/hw_layer",
        L::new(
            "Capa de contactos de techo",
            "Hanging-wall contact layer",
            "Camada de contatos de capa",
        ),
        "Layer of the mapped hanging-wall contact polylines (wildcards * ? allowed). Empty = none.",
    ),
    setting(
        "fw_layer",
        Kind::Text,
        "/vn/fw_layer",
        L::new(
            "Capa de contactos de piso",
            "Footwall contact layer",
            "Camada de contatos de lapa",
        ),
        "Layer of the mapped footwall contact polylines (wildcards allowed). Empty = none.",
    ),
    setting(
        "intercepts",
        Kind::Bool,
        "/vn/use_intercepts",
        L::new(
            "Usar intersecciones de sondajes y canales",
            "Use drillhole and channel intercepts",
            "Usar interseções de furos e canais",
        ),
        "Also use the vein intervals of the loaded drillholes and channels as hanging-wall / footwall contacts.",
    ),
    setting(
        "vein_attr",
        Kind::Text,
        "/vn/vein_attr",
        L::new("Atributo de geología", "Geology attribute", "Atributo de geologia"),
        "Geology attribute that marks the vein intervals (e.g. tipo, lito).",
    ),
    setting(
        "vein_values",
        Kind::Text,
        "/vn/vein_values",
        L::new("Valores de veta (a;b)", "Vein values (a;b)", "Valores de veio (a;b)"),
        "Values of that attribute that are vein, separated by ; (e.g. VETA).",
    ),
    setting(
        "orientation",
        Kind::Choice { options: ORIENT_MODES },
        "/vn/orient_mode",
        L::new("Plano de referencia", "Reference plane", "Plano de referência"),
        "Plane of the vein frame: fitted to all contacts (least squares) or the dip / dip direction typed.",
    ),
    setting(
        "dip",
        Kind::num(Some(0.0), Some(90.0), 5.0),
        "/vn/dip",
        L::new("Buzamiento", "Dip", "Mergulho"),
        "Dip of the reference plane (orientation=typed).",
    ),
    setting(
        "dipdir",
        Kind::num(Some(0.0), Some(360.0), 5.0),
        "/vn/dipdir",
        L::new("Dirección de buzamiento", "Dip direction", "Direção de mergulho"),
        "Dip direction of the reference plane (orientation=typed).",
    ),
    setting(
        "step",
        Kind::num(Some(0.1), None, 0.5),
        "/vn/step",
        L::new("Paso de la malla (m)", "Grid step (m)", "Passo da malha (m)"),
        "Grid spacing on the vein plane.",
    ),
    setting(
        "margin",
        Kind::num(Some(0.0), None, 1.0),
        "/vn/margin",
        L::new("Margen (m)", "Margin (m)", "Margem (m)"),
        "Margin of the grid around the contacts.",
    ),
    setting(
        "radius",
        Kind::num(Some(0.1), None, 5.0),
        "/vn/radius",
        L::new("Radio de búsqueda (m)", "Search radius (m)", "Raio de busca (m)"),
        "Search radius; grid nodes farther from the contacts stay empty (maximum extrapolation).",
    ),
    setting(
        "min_contacts",
        Kind::Integer {
            min: Some(1),
            max: Some(50),
        },
        "/vn/min_samples",
        L::new("Mínimo de contactos", "Minimum contacts", "Mínimo de contatos"),
        "Minimum contact points to estimate a node.",
    ),
    setting(
        "max_contacts",
        Kind::Integer {
            min: Some(1),
            max: Some(200),
        },
        "/vn/max_samples",
        L::new("Máximo de contactos", "Maximum contacts", "Máximo de contatos"),
        "Maximum contact points used per node.",
    ),
    setting(
        "method",
        Kind::Choice { options: METHODS },
        "/vn/method",
        L::new("Interpolación", "Interpolation", "Interpolação"),
        "Interpolation of the walls.",
    ),
    setting(
        "power",
        Kind::num(Some(0.1), None, 0.5),
        "/vn/power",
        L::new("Potencia IDW", "IDW power", "Potência IDW"),
        "Inverse distance power (method=idw).",
    ),
    setting(
        "range",
        Kind::num(Some(0.0), None, 5.0),
        "/vn/range",
        L::new(
            "Alcance del variograma (m, 0 = radio)",
            "Variogram range (m, 0 = radius)",
            "Alcance do variograma (m, 0 = raio)",
        ),
        "Range of the spherical model (method=kriging); 0 uses the search radius.",
    ),
    setting(
        "densify",
        Kind::num(Some(0.0), None, 0.5),
        "/vn/densify",
        L::new(
            "Densificar contactos cada (m)",
            "Densify contacts every (m)",
            "Densificar contatos a cada (m)",
        ),
        "Points taken along the mapped polylines at least every this distance (0 = vertices only).",
    ),
    setting(
        "min_thickness",
        Kind::num(Some(0.0), None, 0.05),
        "/vn/min_thickness",
        L::new(
            "Potencia mínima esperada (m)",
            "Minimum expected thickness (m)",
            "Espessura mínima esperada (m)",
        ),
        "Nodes thinner than this are reported as thin (diagnostic only).",
    ),
    input(
        "hw",
        Kind::Handles,
        false,
        L::new("Polilíneas de techo", "Hanging-wall polylines", "Polilinhas de capa"),
        "Hanging-wall contact polylines picked instead of (or in addition to) the layer.",
    ),
    input(
        "fw",
        Kind::Handles,
        false,
        L::new("Polilíneas de piso", "Footwall polylines", "Polilinhas de lapa"),
        "Footwall contact polylines picked instead of (or in addition to) the layer.",
    ),
];

pub static OPS: &[OpDef] = &[OpDef {
    name: "vn_surfaces",
    command: "MIN_VN_SURFACES",
    aliases: &[],
    label: L::new("Construir superficies", "Build surfaces", "Construir superfícies"),
    group: Group::Veins,
    kind: OpKind::Action,
    summary: "Build the hanging-wall and footwall surfaces of a narrow vein from the mapped contact polylines (by layer, or picked as hw= / fw=) and from the vein intercepts of the loaded drillholes and channels; they are interpolated in the vein frame (strike, dip, normal), drawn and saved, and diagnosed: fit to each contact (worst first), nodes where the footwall crosses above the hanging wall, nodes with one wall only, thickness range. Read the diagnostics and adjust step, radius, method or the contacts, then build again.",
    params: SURFACES,
    positional: &["vein"],
    result: "`vein`, `plane` (`{dip, dipdir, origin}`), `contacts` (`{hanging_wall, footwall, from_layers, from_handles, from_intercepts}`), `grid` (`{nx, ny, step, nodes}`), `diagnostics` (`{both, one_side, crossings, thin, thickness_min, thickness_mean, thickness_max, rms_hw, rms_fw, unchecked}`), `worst` (list of `{source, side, residual, point}`), `file` (saved surfaces or null).",
}];

pub static PANELS: &[PanelSpec] = &[PanelSpec {
    id: "min_vein",
    title: L::new("Veta: superficies", "Vein: surfaces", "Veio: superfícies"),
    command: "MIN_VN_PANEL",
    items: &[
        Item::Param("vn_surfaces", "vein"),
        Item::Section {
            id: "vn_f_contacts",
            title: L::new("Contactos", "Contacts", "Contatos"),
            items: &[
                Item::Param("vn_surfaces", "hw_layer"),
                Item::Param("vn_surfaces", "fw_layer"),
                Item::Param("vn_surfaces", "densify"),
                Item::Param("vn_surfaces", "intercepts"),
                Item::Param("vn_surfaces", "vein_attr"),
                Item::Param("vn_surfaces", "vein_values"),
            ],
            collapsed: false,
        },
        Item::Section {
            id: "vn_f_plane",
            title: L::new("Plano de referencia", "Reference plane", "Plano de referência"),
            items: &[
                Item::Param("vn_surfaces", "orientation"),
                Item::Param("vn_surfaces", "dip"),
                Item::Param("vn_surfaces", "dipdir"),
            ],
            collapsed: true,
        },
        Item::Section {
            id: "vn_f_interp",
            title: L::new("Interpolación", "Interpolation", "Interpolação"),
            items: &[
                Item::Param("vn_surfaces", "step"),
                Item::Param("vn_surfaces", "margin"),
                Item::Param("vn_surfaces", "radius"),
                Item::Param("vn_surfaces", "method"),
                Item::Param("vn_surfaces", "power"),
                Item::Param("vn_surfaces", "range"),
                Item::Param("vn_surfaces", "min_contacts"),
                Item::Param("vn_surfaces", "max_contacts"),
                Item::Param("vn_surfaces", "min_thickness"),
            ],
            collapsed: false,
        },
        Item::Button("vn_surfaces"),
        Item::Status("vn_status"),
    ],
}];

// ---- Stage B2: prism model ------------------------------------------------

const KINDS: &[Opt] = &[
    opt("spherical", L::new("Esférico", "Spherical", "Esférico")),
    opt("exponential", L::new("Exponencial", "Exponential", "Exponencial")),
    opt("gaussian", L::new("Gaussiano", "Gaussian", "Gaussiano")),
];

const COLOR_BY: &[Opt] = &[
    opt("grade", L::new("Ley", "Grade", "Teor")),
    opt("thickness", L::new("Potencia", "Thickness", "Espessura")),
    opt("accumulation", L::new("Acumulación", "Accumulation", "Acumulação")),
    opt(
        "distance",
        L::new("Distancia a la muestra", "Distance to sample", "Distância à amostra"),
    ),
    opt(
        "variance",
        L::new("Varianza de krigeado", "Kriging variance", "Variância de krigagem"),
    ),
];

const EDIT_ACTIONS: &[Opt] = &[
    opt(
        "exclude",
        L::new("Excluir prismas", "Exclude prisms", "Excluir prismas"),
    ),
    opt("grade", L::new("Fijar ley", "Set grade", "Fixar teor")),
    opt(
        "thickness",
        L::new("Fijar potencia", "Set thickness", "Fixar espessura"),
    ),
    opt(
        "boundary",
        L::new("Polígono límite", "Boundary polygon", "Polígono limite"),
    ),
    opt(
        "undo",
        L::new("Deshacer la última", "Undo the last one", "Desfazer a última"),
    ),
    opt(
        "clear",
        L::new(
            "Borrar ediciones y límite",
            "Clear edits and boundary",
            "Apagar edições e limite",
        ),
    ),
    opt("list", L::new("Listar", "List", "Listar")),
];

const VEIN: Param = setting(
    "vein",
    Kind::Text,
    "/vn/name",
    L::new("Veta", "Vein", "Veio"),
    "Name of the vein (its surfaces must be built first).",
);
const GRADE: Param = setting(
    "grade",
    Kind::Text,
    "/vn/grade",
    L::new("Ley (columna)", "Grade (column)", "Teor (coluna)"),
    "Assay column estimated (e.g. au, ag, cu%).",
);
const THICK: Param = setting("thickness_col", Kind::Text, "/vn/thick_col", L::new("Columna de potencia", "Thickness column", "Coluna de espessura"),
    "Assay column with the true vein thickness (channels: potencia). Empty or missing = length of the intercept along the vein normal.");

const VARIO: &[Param] = &[
    VEIN,
    GRADE,
    THICK,
    setting(
        "lag",
        Kind::num(Some(0.1), None, 1.0),
        "/vn/lag",
        L::new("Paso (m)", "Lag (m)", "Passo (m)"),
        "Lag of the experimental variograms on the vein plane.",
    ),
    setting(
        "nlags",
        Kind::Integer {
            min: Some(2),
            max: Some(50),
        },
        "/vn/nlags",
        L::new("Número de pasos", "Number of lags", "Número de passos"),
        "Number of lags.",
    ),
    setting(
        "kind",
        Kind::Choice { options: KINDS },
        "/vn/model_kind",
        L::new("Modelo", "Model", "Modelo"),
        "Variogram model type.",
    ),
];

const ESTIMATE: &[Param] = &[
    VEIN,
    GRADE,
    THICK,
    setting(
        "kind",
        Kind::Choice { options: KINDS },
        "/vn/model_kind",
        L::new("Modelo", "Model", "Modelo"),
        "Variogram model type.",
    ),
    setting(
        "nugget",
        Kind::num(Some(0.0), None, 0.1),
        "/vn/nugget",
        L::new("Efecto pepita", "Nugget", "Efeito pepita"),
        "Nugget of the accumulation variogram (vn_variogram fills it).",
    ),
    setting(
        "sill",
        Kind::num(Some(0.0), None, 0.1),
        "/vn/sill",
        L::new("Meseta", "Sill", "Patamar"),
        "Partial sill of the accumulation variogram.",
    ),
    setting(
        "range_strike",
        Kind::num(Some(0.01), None, 5.0),
        "/vn/range_strike",
        L::new(
            "Alcance en rumbo (m)",
            "Range along strike (m)",
            "Alcance na direção (m)",
        ),
        "Range along strike.",
    ),
    setting(
        "range_dip",
        Kind::num(Some(0.01), None, 5.0),
        "/vn/range_dip",
        L::new(
            "Alcance en buzamiento (m)",
            "Range down dip (m)",
            "Alcance no mergulho (m)",
        ),
        "Range down dip.",
    ),
    setting(
        "block_u",
        Kind::num(Some(0.1), None, 1.0),
        "/vn/block_u",
        L::new("Prisma en rumbo (m)", "Prism along strike (m)", "Prisma na direção (m)"),
        "Prism size along strike.",
    ),
    setting(
        "block_v",
        Kind::num(Some(0.1), None, 1.0),
        "/vn/block_v",
        L::new(
            "Prisma en buzamiento (m)",
            "Prism down dip (m)",
            "Prisma no mergulho (m)",
        ),
        "Prism size down dip.",
    ),
    setting(
        "search_radius",
        Kind::num(Some(0.1), None, 5.0),
        "/vn/est_radius",
        L::new("Radio de búsqueda (m)", "Search radius (m)", "Raio de busca (m)"),
        "Search radius on the vein plane.",
    ),
    setting(
        "min_samples",
        Kind::Integer {
            min: Some(1),
            max: Some(50),
        },
        "/vn/est_min",
        L::new("Mínimo de muestras", "Minimum samples", "Mínimo de amostras"),
        "Minimum composites to estimate a prism.",
    ),
    setting(
        "max_samples",
        Kind::Integer {
            min: Some(1),
            max: Some(200),
        },
        "/vn/est_max",
        L::new("Máximo de muestras", "Maximum samples", "Máximo de amostras"),
        "Maximum composites per prism.",
    ),
    setting(
        "max_distance",
        Kind::num(Some(0.0), None, 5.0),
        "/vn/max_dist",
        L::new(
            "Distancia máxima (m, 0 = sin límite)",
            "Maximum distance (m, 0 = none)",
            "Distância máxima (m, 0 = sem limite)",
        ),
        "Prisms farther than this from every composite are left out.",
    ),
    setting(
        "disc",
        Kind::Integer {
            min: Some(1),
            max: Some(10),
        },
        "/vn/disc",
        L::new(
            "Discretización por lado",
            "Discretisation per side",
            "Discretização por lado",
        ),
        "Points per side used for block kriging.",
    ),
    setting(
        "density",
        Kind::num(Some(0.1), None, 0.1),
        "/vn/density",
        L::new("Densidad (t/m³)", "Density (t/m³)", "Densidade (t/m³)"),
        "Density of the vein.",
    ),
    setting(
        "color_by",
        Kind::Choice { options: COLOR_BY },
        "/vn/color_by",
        L::new("Colorear por", "Colour by", "Colorir por"),
        "Attribute that colours the prisms and the long section.",
    ),
    setting(
        "draw_prisms",
        Kind::Bool,
        "/vn/draw_prisms",
        L::new("Dibujar prismas 3D", "Draw 3D prisms", "Desenhar prismas 3D"),
        "Draw the prisms between the walls.",
    ),
];

const LONG: &[Param] = &[
    VEIN,
    setting(
        "color_by",
        Kind::Choice { options: COLOR_BY },
        "/vn/color_by",
        L::new("Colorear por", "Colour by", "Colorir por"),
        "Attribute that colours the long section.",
    ),
    setting(
        "labels",
        Kind::Bool,
        "/vn/long_labels",
        L::new("Rótulos de muestras", "Sample labels", "Rótulos das amostras"),
        "Write hole and grade beside each composite.",
    ),
    input(
        "at",
        Kind::Point,
        false,
        L::new("Punto base", "Base point", "Ponto base"),
        "Lower-left corner of the long section in the drawing. Default: the last one, or beside the model.",
    ),
];

const EDIT: &[Param] = &[
    VEIN,
    setting(
        "action",
        Kind::Choice { options: EDIT_ACTIONS },
        "/vn/edit_action",
        L::new("Edición", "Edit", "Edição"),
        "What to do with the polygon drawn on the long section.",
    ),
    input(
        "polygon",
        Kind::Handle,
        false,
        L::new("Polígono", "Polygon", "Polígono"),
        "Closed polyline drawn on the long section (exclude, grade, thickness, boundary).",
    ),
    setting(
        "value",
        Kind::num(None, None, 0.1),
        "/vn/edit_value",
        L::new("Valor", "Value", "Valor"),
        "Grade or thickness for action=grade / thickness.",
    ),
];

pub static MODEL_OPS: &[OpDef] = &[
    OpDef {
        name: "vn_variogram",
        command: "MIN_VN_VARIOGRAM",
        aliases: &[],
        label: L::new("Variografía en el plano", "In-plane variography", "Variografia no plano"),
        group: Group::Veins,
        kind: OpKind::Action,
        summary: "Composites the vein intercepts of the loaded drillholes and channels (grade weighted by length, true thickness from the thickness column), computes the variograms of the accumulation (grade × thickness) on the vein plane (omnidirectional, along strike, down dip), fits a model and stores nugget, sill and both ranges for vn_estimate.",
        params: VARIO,
        positional: &["vein"],
        result: "`vein`, `composites`, `missing_grade`, `stats` (`{grade_mean, thickness_mean, acc_mean, acc_variance}`), `experimental` (`{omni, strike, dip}`: lists of `{lag, gamma, pairs}`), `model` (`{kind, nugget, sill, range_strike, range_dip}`).",
    },
    OpDef {
        name: "vn_estimate",
        command: "MIN_VN_ESTIMATE",
        aliases: &[],
        label: L::new("Estimar prismas", "Estimate prisms", "Estimar prismas"),
        group: Group::Veins,
        kind: OpKind::Action,
        summary: "Builds the prism model of the vein: a grid of prisms on the vein plane between the hanging-wall and footwall surfaces; accumulation and thickness are block-kriged with the same weights and grade = accumulation / thickness. The boundary polygon, the maximum distance and the long-section edits limit or override the prisms. Draws the prisms, saves them as CSV, redraws the long section if drawn, and validates against the composites (cross-validation and composite vs prism grade).",
        params: ESTIMATE,
        positional: &["vein"],
        result: "`vein`, `summary` (`{prisms, area, volume, tonnes, grade, thickness, edited, out_boundary, out_distance, out_walls, out_edits, not_estimated}`), `validation` (`{cv_n, cv_mean_error, cv_correlation, n, mean_composite, mean_prism, correlation, worst: [{hole, composite, prism}]}`), `composites`, `file`.",
    },
    OpDef {
        name: "vn_long",
        command: "MIN_VN_LONG",
        aliases: &[],
        label: L::new("Longitudinal", "Long section", "Longitudinal"),
        group: Group::Veins,
        kind: OpKind::Action,
        summary: "Draws the long section of the vein model flat in the drawing (along strike × up dip): prisms coloured by the chosen attribute, composites with labels, the boundary and the edit polygons. Polygons drawn on it are used by vn_edit.",
        params: LONG,
        positional: &["vein"],
        result: "`vein`, `at` (base point), `u0`, `v0`, `width`, `height`, `prisms`, `composites`.",
    },
    OpDef {
        name: "vn_edit",
        command: "MIN_VN_EDIT",
        aliases: &[],
        label: L::new("Editar en la longitudinal", "Edit on the long section", "Editar na longitudinal"),
        group: Group::Veins,
        kind: OpKind::Action,
        summary: "Edits the prism model with a polygon drawn on the long section (polygon=handle, else the selection, else it asks to pick it): exclude prisms, set their grade or thickness, or set the boundary polygon; undo the last edit, clear all, or list them. The model is estimated again after each change.",
        params: EDIT,
        positional: &["action"],
        result: "`vein`, `edits` (list of `{action, value, vertices}`), `boundary` (vertex count), and the vn_estimate result in `estimate` (null for list).",
    },
];

pub static MODEL_PANELS: &[PanelSpec] = &[PanelSpec {
    id: "min_vein_model",
    title: L::new(
        "Veta: modelo de prismas",
        "Vein: prism model",
        "Veio: modelo de prismas",
    ),
    command: "MIN_VN_MODEL_PANEL",
    items: &[
        Item::Param("vn_estimate", "vein"),
        Item::Param("vn_estimate", "grade"),
        Item::Param("vn_estimate", "thickness_col"),
        Item::Section {
            id: "vn_f_vario",
            title: L::new(
                "Variografía en el plano",
                "In-plane variography",
                "Variografia no plano",
            ),
            items: &[
                Item::Param("vn_variogram", "lag"),
                Item::Param("vn_variogram", "nlags"),
                Item::Param("vn_estimate", "kind"),
                Item::Button("vn_variogram"),
                Item::Chart("vn_vario_chart"),
                Item::Param("vn_estimate", "nugget"),
                Item::Param("vn_estimate", "sill"),
                Item::Param("vn_estimate", "range_strike"),
                Item::Param("vn_estimate", "range_dip"),
            ],
            collapsed: false,
        },
        Item::Section {
            id: "vn_f_prisms",
            title: L::new("Prismas", "Prisms", "Prismas"),
            items: &[
                Item::Param("vn_estimate", "block_u"),
                Item::Param("vn_estimate", "block_v"),
                Item::Param("vn_estimate", "search_radius"),
                Item::Param("vn_estimate", "min_samples"),
                Item::Param("vn_estimate", "max_samples"),
                Item::Param("vn_estimate", "max_distance"),
                Item::Param("vn_estimate", "disc"),
                Item::Param("vn_estimate", "density"),
                Item::Param("vn_estimate", "color_by"),
                Item::Param("vn_estimate", "draw_prisms"),
            ],
            collapsed: false,
        },
        Item::Button("vn_estimate"),
        Item::Section {
            id: "vn_f_long",
            title: L::new(
                "Longitudinal y ediciones",
                "Long section and edits",
                "Longitudinal e edições",
            ),
            items: &[
                Item::Param("vn_long", "labels"),
                Item::Button("vn_long"),
                Item::Param("vn_edit", "action"),
                Item::Param("vn_edit", "value"),
                Item::Button("vn_edit"),
            ],
            collapsed: true,
        },
        Item::Status("vn_model_status"),
    ],
}];

// ---- Stage B3: tonnage, blocks and categories -------------------------------

const RULES: Param = setting(
    "rules",
    Kind::Table {
        columns: &[
            L::new("Categoría", "Category", "Categoria"),
            L::new("Lados muestreados ≥", "Sampled sides ≥", "Lados amostrados ≥"),
            L::new("Distancia ≤ (m)", "Distance ≤ (m)", "Distância ≤ (m)"),
            L::new("Muestras ≥", "Samples ≥", "Amostras ≥"),
            L::new("Ley diluida ≥", "Diluted grade ≥", "Teor diluído ≥"),
        ],
        codec: "vn_rules",
    },
    "/vn/rules",
    L::new("Reglas de categorización", "Category rules", "Regras de categorização"),
    "Ordered rules [category, min sampled sides, max mean distance, min samples, min diluted grade]; empty cells are not checked. Each block takes the first rule it meets; blocks that meet none are unclassified. Replace the example rules with the company's norms.",
);

const CUBICATE: &[Param] = &[
    VEIN,
    setting(
        "mmw",
        Kind::num(Some(0.0), None, 0.05),
        "/vn/mmw",
        L::new(
            "Ancho mínimo de minado (m)",
            "Minimum mining width (m)",
            "Largura mínima de lavra (m)",
        ),
        "Thinner prisms are widened to this width, half on each wall.",
    ),
    setting(
        "overbreak_hw",
        Kind::num(Some(0.0), None, 0.05),
        "/vn/ob_hw",
        L::new(
            "Sobre-excavación techo (m)",
            "Hanging-wall overbreak (m)",
            "Sobre-escavação capa (m)",
        ),
        "Overbreak on the hanging wall.",
    ),
    setting(
        "overbreak_fw",
        Kind::num(Some(0.0), None, 0.05),
        "/vn/ob_fw",
        L::new(
            "Sobre-excavación piso (m)",
            "Footwall overbreak (m)",
            "Sobre-escavação lapa (m)",
        ),
        "Overbreak on the footwall.",
    ),
    setting(
        "grade_hw",
        Kind::num(Some(0.0), None, 0.1),
        "/vn/grade_hw",
        L::new(
            "Ley de la caja techo",
            "Hanging-wall rock grade",
            "Teor da encaixante capa",
        ),
        "Grade of the diluting rock on the hanging wall.",
    ),
    setting(
        "grade_fw",
        Kind::num(Some(0.0), None, 0.1),
        "/vn/grade_fw",
        L::new("Ley de la caja piso", "Footwall rock grade", "Teor da encaixante lapa"),
        "Grade of the diluting rock on the footwall.",
    ),
    setting(
        "density_waste",
        Kind::num(Some(0.1), None, 0.1),
        "/vn/density_waste",
        L::new(
            "Densidad de la caja (t/m³)",
            "Wall rock density (t/m³)",
            "Densidade da encaixante (t/m³)",
        ),
        "Density of the diluting rock.",
    ),
    setting(
        "recovery",
        Kind::num(Some(0.0), Some(100.0), 1.0),
        "/vn/recovery",
        L::new("Recuperación (%)", "Recovery (%)", "Recuperação (%)"),
        "Mining recovery applied to the diluted tonnes.",
    ),
    setting(
        "level_layer",
        Kind::Text,
        "/vn/level_layer",
        L::new(
            "Capa de niveles (galerías)",
            "Level layer (drifts)",
            "Camada de níveis (galerias)",
        ),
        "Layer of the level polylines (wildcards allowed). Empty = none.",
    ),
    setting(
        "raise_layer",
        Kind::Text,
        "/vn/raise_layer",
        L::new("Capa de chimeneas", "Raise layer", "Camada de chaminés"),
        "Layer of the raise polylines (wildcards allowed). Empty = none.",
    ),
    setting(
        "level_pillar",
        Kind::num(Some(0.0), None, 0.5),
        "/vn/level_pillar",
        L::new(
            "Puente sobre cada nivel (m)",
            "Sill pillar above each level (m)",
            "Pilar sobre cada nível (m)",
        ),
        "Height of the pillar left above each level, along the vein (0 = none).",
    ),
    setting(
        "raise_pillar",
        Kind::num(Some(0.0), None, 0.5),
        "/vn/raise_pillar",
        L::new("Pilar en chimeneas (m)", "Raise pillar (m)", "Pilar nas chaminés (m)"),
        "Total width of the pillar centred on each raise (0 = none).",
    ),
    setting(
        "side_tolerance",
        Kind::num(Some(0.0), None, 0.5),
        "/vn/side_tol",
        L::new(
            "Tolerancia de lado muestreado (m)",
            "Sampled side tolerance (m)",
            "Tolerância de lado amostrado (m)",
        ),
        "A composite within this distance of a working samples that side of the block.",
    ),
    setting(
        "side_min",
        Kind::Integer {
            min: Some(1),
            max: Some(100),
        },
        "/vn/side_min",
        L::new("Muestras por lado", "Samples per side", "Amostras por lado"),
        "Composites needed on a side to count it as sampled.",
    ),
    RULES,
    input(
        "levels",
        Kind::Handles,
        false,
        L::new("Niveles", "Levels", "Níveis"),
        "Level (drift) polylines picked instead of (or in addition to) the layer.",
    ),
    input(
        "raises",
        Kind::Handles,
        false,
        L::new("Chimeneas", "Raises", "Chaminés"),
        "Raise polylines picked instead of (or in addition to) the layer.",
    ),
];

const RULES_TEST: &[Param] = &[
    RULES,
    input(
        "sides",
        Kind::Integer {
            min: Some(0),
            max: Some(4),
        },
        false,
        L::new("Lados muestreados", "Sampled sides", "Lados amostrados"),
        "Sampled sides of the block to test.",
    ),
    input(
        "dist",
        Kind::num(Some(0.0), None, 1.0),
        false,
        L::new("Distancia (m)", "Distance (m)", "Distância (m)"),
        "Mean distance to the samples to test.",
    ),
    input(
        "samples",
        Kind::Integer {
            min: Some(0),
            max: None,
        },
        false,
        L::new("Muestras", "Samples", "Amostras"),
        "Samples of the block to test.",
    ),
    input(
        "grade",
        Kind::num(None, None, 0.1),
        false,
        L::new("Ley diluida", "Diluted grade", "Teor diluído"),
        "Diluted grade to test.",
    ),
];

const fn same(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    let mut i = 0;
    while i < a.len() {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}

/// The parameter `key` of a list (a compile error when it is missing).
const fn pick(list: &[Param], key: &str) -> Param {
    let mut i = 0;
    while i < list.len() {
        if same(list[i].key, key) {
            return list[i];
        }
        i += 1;
    }
    panic!("no such parameter in the list")
}

/// `vn_build` accepts every setting of the steps it chains (surfaces, variogram,
/// estimate, tonnage), so one call can set and run them all; a test checks that
/// none is missing.
const BUILD: &[Param] = &[
    VEIN,
    setting(
        "fit_variogram",
        Kind::Bool,
        "/vn/build_fit",
        L::new("Ajustar el variograma", "Fit the variogram", "Ajustar o variograma"),
        "vn_build fits the in-plane variogram before estimating (otherwise uses the stored model: nugget, sill, range_strike, range_dip).",
    ),
    setting(
        "tonnage",
        Kind::Bool,
        "/vn/build_cubicate",
        L::new("Cubicar", "Compute tonnage", "Cubar"),
        "vn_build ends with vn_cubicate.",
    ),
    // vn_surfaces
    pick(SURFACES, "hw_layer"),
    pick(SURFACES, "fw_layer"),
    pick(SURFACES, "intercepts"),
    pick(SURFACES, "vein_attr"),
    pick(SURFACES, "vein_values"),
    pick(SURFACES, "orientation"),
    pick(SURFACES, "dip"),
    pick(SURFACES, "dipdir"),
    pick(SURFACES, "step"),
    pick(SURFACES, "margin"),
    pick(SURFACES, "radius"),
    pick(SURFACES, "min_contacts"),
    pick(SURFACES, "max_contacts"),
    pick(SURFACES, "method"),
    pick(SURFACES, "power"),
    pick(SURFACES, "range"),
    pick(SURFACES, "densify"),
    pick(SURFACES, "min_thickness"),
    pick(SURFACES, "hw"),
    pick(SURFACES, "fw"),
    // vn_variogram
    pick(VARIO, "grade"),
    pick(VARIO, "thickness_col"),
    pick(VARIO, "lag"),
    pick(VARIO, "nlags"),
    pick(VARIO, "kind"),
    // vn_estimate
    pick(ESTIMATE, "nugget"),
    pick(ESTIMATE, "sill"),
    pick(ESTIMATE, "range_strike"),
    pick(ESTIMATE, "range_dip"),
    pick(ESTIMATE, "block_u"),
    pick(ESTIMATE, "block_v"),
    pick(ESTIMATE, "search_radius"),
    pick(ESTIMATE, "min_samples"),
    pick(ESTIMATE, "max_samples"),
    pick(ESTIMATE, "max_distance"),
    pick(ESTIMATE, "disc"),
    pick(ESTIMATE, "density"),
    pick(ESTIMATE, "color_by"),
    pick(ESTIMATE, "draw_prisms"),
    // vn_cubicate
    pick(CUBICATE, "mmw"),
    pick(CUBICATE, "overbreak_hw"),
    pick(CUBICATE, "overbreak_fw"),
    pick(CUBICATE, "grade_hw"),
    pick(CUBICATE, "grade_fw"),
    pick(CUBICATE, "density_waste"),
    pick(CUBICATE, "recovery"),
    pick(CUBICATE, "level_layer"),
    pick(CUBICATE, "raise_layer"),
    pick(CUBICATE, "level_pillar"),
    pick(CUBICATE, "raise_pillar"),
    pick(CUBICATE, "side_tolerance"),
    pick(CUBICATE, "side_min"),
    pick(CUBICATE, "rules"),
    pick(CUBICATE, "levels"),
    pick(CUBICATE, "raises"),
];

pub static TONNAGE_OPS: &[OpDef] = &[
    OpDef {
        name: "vn_cubicate",
        command: "MIN_VN_CUBICATE",
        aliases: &[],
        label: L::new("Cubicar", "Compute tonnage", "Cubar"),
        group: Group::Veins,
        kind: OpKind::Action,
        summary: "Tonnage of the estimated prism model: dilution (minimum mining width, hanging-wall / footwall overbreak with their grades) and recovery per prism; blocks between the levels (drifts) and raises picked or on their layers, projected on the vein plane, with sill and raise pillars kept apart; sampled sides per block (composites on the bounding workings); categories by the ordered rule table. Draws the workings and block labels on the long section and saves the blocks as CSV.",
        params: CUBICATE,
        positional: &["vein"],
        result: "`vein`, `workings` (`{levels, raises}`), `blocks` (list of `{name, category, sides: [lower, upper, left, right] (true / false / null), sampled, samples, dist, prisms, area, thickness, tonnes, grade, width, diluted_tonnes, diluted_grade, recovered, pillar_tonnes, why}`), `totals` (per category `{category, blocks, tonnes, grade, diluted_tonnes, diluted_grade, recovered}`, last = unclassified), `pillars` (`{tonnes, grade}`), `file`.",
    },
    OpDef {
        name: "vn_rules",
        command: "MIN_VN_RULES",
        aliases: &[],
        label: L::new("Probar reglas", "Test rules", "Testar regras"),
        group: Group::Veins,
        kind: OpKind::Action,
        summary: "Lists the category rules (set them with rules=[[category, min_sides, max_dist, min_samples, min_grade], ...]) and, given sides / dist / samples / grade, tells which rule a block with those values meets and why each earlier rule fails.",
        params: RULES_TEST,
        positional: &[],
        result: "`rules` (list of `{category, min_sides, max_dist, min_samples, min_grade}`), `test` (`{values, rule, category, why}` or null).",
    },
    OpDef {
        name: "vn_build",
        command: "MIN_VN_BUILD",
        aliases: &[],
        label: L::new("Construir todo", "Build all", "Construir tudo"),
        group: Group::Veins,
        kind: OpKind::Action,
        summary: "Runs the whole vein model: vn_surfaces, vn_variogram (if fit_variogram), vn_estimate and vn_cubicate (if tonnage). It accepts every argument of those steps (set them in the same call: contacts, step, radius, grade, block sizes, maximum distance, dilution, workings, rules…; omitted ones use the current settings) and stops at the first step that fails, naming it; otherwise returns every step's result.",
        params: BUILD,
        positional: &["vein"],
        result: "`vein`, `steps` (`{surfaces, variogram, estimate, cubicate}`: each step's data or null when skipped).",
    },
];

pub static TONNAGE_PANELS: &[PanelSpec] = &[PanelSpec {
    id: "min_vein_tonnage",
    title: L::new("Veta: cubicación", "Vein: tonnage", "Veio: cubagem"),
    command: "MIN_VN_TONNAGE_PANEL",
    items: &[
        Item::Param("vn_cubicate", "vein"),
        Item::Section {
            id: "vn_f_dilution",
            title: L::new(
                "Dilución y recuperación",
                "Dilution and recovery",
                "Diluição e recuperação",
            ),
            items: &[
                Item::Param("vn_cubicate", "mmw"),
                Item::Param("vn_cubicate", "overbreak_hw"),
                Item::Param("vn_cubicate", "overbreak_fw"),
                Item::Param("vn_cubicate", "grade_hw"),
                Item::Param("vn_cubicate", "grade_fw"),
                Item::Param("vn_cubicate", "density_waste"),
                Item::Param("vn_cubicate", "recovery"),
            ],
            collapsed: false,
        },
        Item::Section {
            id: "vn_f_blocks",
            title: L::new(
                "Bloques entre labores",
                "Blocks between workings",
                "Blocos entre labores",
            ),
            items: &[
                Item::Param("vn_cubicate", "level_layer"),
                Item::Param("vn_cubicate", "raise_layer"),
                Item::Param("vn_cubicate", "level_pillar"),
                Item::Param("vn_cubicate", "raise_pillar"),
                Item::Param("vn_cubicate", "side_tolerance"),
                Item::Param("vn_cubicate", "side_min"),
            ],
            collapsed: false,
        },
        Item::Section {
            id: "vn_f_rules",
            title: L::new("Categorías", "Categories", "Categorias"),
            items: &[Item::Param("vn_cubicate", "rules")],
            collapsed: false,
        },
        Item::Button("vn_cubicate"),
        Item::Section {
            id: "vn_f_build",
            title: L::new("Construir todo", "Build all", "Construir tudo"),
            items: &[
                Item::Param("vn_build", "fit_variogram"),
                Item::Param("vn_build", "tonnage"),
                Item::Button("vn_build"),
            ],
            collapsed: true,
        },
        Item::Status("vn_tonnage_status"),
    ],
}];

#[cfg(test)]
mod tests {
    use super::*;

    /// `vn_build` must accept every setting of the steps it chains, under the same key and setting.
    #[test]
    fn vn_build_accepts_every_step_setting() {
        let all: Vec<&OpDef> = OPS.iter().chain(MODEL_OPS).chain(TONNAGE_OPS).collect();
        let build = all.iter().find(|o| o.name == "vn_build").unwrap();
        for step in ["vn_surfaces", "vn_variogram", "vn_estimate", "vn_cubicate"] {
            let def = all.iter().find(|o| o.name == step).unwrap();
            for p in def.params.iter().filter(|p| p.setting.is_some()) {
                let b = build
                    .param(p.key)
                    .unwrap_or_else(|| panic!("vn_build lacks {step}.{}", p.key));
                assert_eq!(b.setting, p.setting, "{step}.{}", p.key);
            }
        }
        // Keys are unique inside every operation.
        for o in &all {
            let mut keys: Vec<&str> = o.params.iter().map(|p| p.key).collect();
            keys.sort();
            keys.dedup();
            assert_eq!(keys.len(), o.params.len(), "{}", o.name);
        }
    }
}
