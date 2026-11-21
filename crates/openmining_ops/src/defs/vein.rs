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
        "min_samples",
        Kind::Integer {
            min: Some(1),
            max: Some(50),
        },
        "/vn/min_samples",
        L::new("Mínimo de contactos", "Minimum contacts", "Mínimo de contatos"),
        "Minimum contact points to estimate a node.",
    ),
    setting(
        "max_samples",
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
                Item::Param("vn_surfaces", "min_samples"),
                Item::Param("vn_surfaces", "max_samples"),
                Item::Param("vn_surfaces", "min_thickness"),
            ],
            collapsed: false,
        },
        Item::Button("vn_surfaces"),
        Item::Status("vn_status"),
    ],
}];
