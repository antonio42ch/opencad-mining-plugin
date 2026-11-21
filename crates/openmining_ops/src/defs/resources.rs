//! Resource estimation: composites, variography, block model, views.

#[allow(unused_imports)]
use crate::{input, opt, setting, Group, Item, Kind, OpDef, OpKind, PanelSpec, Param, L, TABLE_FILES};

#[allow(dead_code)]
const FILES: Kind = Kind::Path {
    extensions: TABLE_FILES,
};

const RS_COMPOSITE: &[Param] = &[setting("comp_length", Kind::num(Some(0.01), None, 1.0), "/rs/comp_length", L::new("Longitud (m)", "Length (m)", "Comprimento (m)"), "Composite length (m)."), setting("comp_remainder", Kind::Choice { options: &[opt("merge", L::new("Unir al anterior", "Merge with previous", "Unir ao anterior")), opt("discard", L::new("Descartar", "Discard", "Descartar")), opt("keep", L::new("Conservar", "Keep", "Manter"))] }, "/rs/comp_remainder", L::new("Tramo final corto", "Short final piece", "Trecho final curto"), "What to do with the short last composite."), setting("comp_min_fraction", Kind::num(Some(0.0), None, 0.1), "/rs/comp_min_fraction", L::new("Fracción mínima del tramo final", "Minimum final-piece fraction", "Fração mínima do trecho final"), "Minimum fraction of the length for a remainder to be kept."), setting("comp_min_coverage", Kind::num(Some(0.0), None, 0.1), "/rs/comp_min_coverage", L::new("Cobertura mínima de muestreo", "Minimum sampled coverage", "Cobertura mínima de amostragem"), "Minimum sampled fraction of a composite."), setting("unit", Kind::Text, "/rs/unit", L::new("Unidad de ley", "Grade unit", "Unidade de teor"), "Grade unit for reports (%, g/t...)."), setting("domains", Kind::Table { columns: &[L::new("Dominio", "Domain", "Domínio"), L::new("Valores de geología (a;b)", "Geology values (a;b)", "Valores de geologia (a;b)"), L::new("Densidad t/m³", "Density t/m³", "Densidade t/m³"), L::new("Tope de ley", "Top-cut", "Corte de teor"), L::new("Superficie", "Surface", "Superfície")], codec: "domains" }, "/rs/domains", L::new("Dominios", "Domains", "Domínios"), "Estimation domains: rows of [name, geology values a;b, density t/m3, top-cut (0 none), surface condition ('Above C1', 'Below C1; Above C2')]. Empty table = one Global domain."), setting("dc_min", Kind::num(Some(0.1), None, 5.0), "/rs/dc_min", L::new("Celda mínima (m)", "Minimum cell (m)", "Célula mínima (m)"), "Smallest declustering cell (m)."), setting("dc_max", Kind::num(Some(0.1), None, 10.0), "/rs/dc_max", L::new("Celda máxima (m)", "Maximum cell (m)", "Célula máxima (m)"), "Largest declustering cell (m)."), setting("dc_steps", Kind::num(Some(1.0), None, 1.0), "/rs/dc_steps", L::new("Tamaños a probar", "Sizes to try", "Tamanhos a testar"), "Number of cell sizes tried."), setting("dc_zratio", Kind::num(Some(0.01), None, 0.1), "/rs/dc_zratio", L::new("Razón vertical de la celda", "Vertical cell ratio", "Razão vertical da célula"), "Vertical / horizontal cell ratio.")];

const RS_DRAWCOMP: &[Param] = &[];

const RS_SETSOLID: &[Param] = &[
    setting(
        "target",
        Kind::Index {
            source: "domains",
            all: None,
        },
        "/rs/target",
        L::new(
            "Dominio de los sólidos seleccionados",
            "Domain of the selected solids",
            "Domínio dos sólidos selecionados",
        ),
        "Domain the solids belong to (name or index).",
    ),
    input(
        "solids",
        Kind::Handles,
        false,
        L::new("Sólidos", "Solids", "Sólidos"),
        "Closed meshes. Without it: the selection, or a pick.",
    ),
];

const RS_CONTACT: &[Param] = &[
    setting(
        "contacts",
        Kind::Table {
            columns: &[
                L::new("Superficie", "Surface", "Superfície"),
                L::new("Geología encima", "Geology above", "Geologia acima"),
                L::new("Geología debajo", "Geology below", "Geologia abaixo"),
            ],
            codec: "contacts",
        },
        "/rs/contacts",
        L::new("Superficie de contacto", "Contact surface", "Superfície de contato"),
        "Contact surfaces: rows of [name, geology above, geology below].",
    ),
    setting(
        "step",
        Kind::num(Some(0.1), None, 5.0),
        "/rs/contact/step",
        L::new("Paso de la malla (m)", "Grid step (m)", "Passo da malha (m)"),
        "Grid step of the surfaces (m).",
    ),
    setting(
        "kind",
        Kind::Choice {
            options: &[
                opt("spherical", L::new("Esférico", "Spherical", "Esférico")),
                opt("exponential", L::new("Exponencial", "Exponential", "Exponencial")),
                opt("gaussian", L::new("Gaussiano", "Gaussian", "Gaussiano")),
            ],
        },
        "/rs/contact/kind",
        L::new("Tipo de modelo", "Model type", "Tipo de modelo"),
        "Variogram type of the surface kriging.",
    ),
    setting(
        "nugget",
        Kind::num(Some(0.0), None, 1.0),
        "/rs/contact/nugget",
        L::new("Pepita", "Nugget", "Pepita"),
        "Nugget of the surface variogram.",
    ),
    setting(
        "sill",
        Kind::num(Some(0.0), None, 10.0),
        "/rs/contact/sill",
        L::new("Meseta", "Sill", "Patamar"),
        "Sill of the surface variogram.",
    ),
    setting(
        "range",
        Kind::num(Some(1.0), None, 10.0),
        "/rs/contact/range",
        L::new("Alcance (m)", "Range (m)", "Alcance (m)"),
        "Range of the surface variogram (m).",
    ),
    setting(
        "radius",
        Kind::num(Some(1.0), None, 10.0),
        "/rs/contact/radius",
        L::new("Radio de búsqueda (m)", "Search radius (m)", "Raio de busca (m)"),
        "Search radius (m).",
    ),
    setting(
        "min_samples",
        Kind::num(Some(1.0), None, 1.0),
        "/rs/contact/min_samples",
        L::new("Mínimo de puntos", "Minimum points", "Mínimo de pontos"),
        "Minimum contact points per node.",
    ),
    setting(
        "max_samples",
        Kind::num(Some(1.0), None, 1.0),
        "/rs/contact/max_samples",
        L::new("Máximo de puntos", "Maximum points", "Máximo de pontos"),
        "Maximum contact points per node.",
    ),
];

const RS_VARIO: &[Param] = &[
    setting(
        "domain",
        Kind::Index {
            source: "domains",
            all: None,
        },
        "/rs/vario/domain",
        L::new("Dominio", "Domain", "Domínio"),
        "Estimation domain worked on (name or index).",
    ),
    setting(
        "azimuth",
        Kind::num(None, None, 5.0),
        "/rs/vario/work/azimuth",
        L::new("Azimut del eje mayor", "Major axis azimuth", "Azimute do eixo maior"),
        "Azimuth of the major direction (experimental and model).",
    ),
    setting(
        "dip",
        Kind::num(None, None, 5.0),
        "/rs/vario/work/dip",
        L::new("Inclinación del eje mayor", "Major axis dip", "Mergulho do eixo maior"),
        "Plunge of the major direction, positive down.",
    ),
    setting(
        "rake",
        Kind::num(None, None, 5.0),
        "/rs/vario/work/rake",
        L::new("Giro (rake)", "Rake", "Giro (rake)"),
        "Rotation of the semi-major / minor axes.",
    ),
    setting(
        "lag",
        Kind::num(Some(0.1), None, 5.0),
        "/rs/vario/lag",
        L::new("Paso (m)", "Lag (m)", "Passo (m)"),
        "Lag distance (m).",
    ),
    setting(
        "nlags",
        Kind::num(Some(1.0), None, 1.0),
        "/rs/vario/nlags",
        L::new("N° de pasos", "Number of lags", "N° de passos"),
        "Number of lags.",
    ),
    setting(
        "az_tol",
        Kind::num(Some(0.0), None, 2.5),
        "/rs/vario/az_tol",
        L::new(
            "Tolerancia azimut (°)",
            "Azimuth tolerance (°)",
            "Tolerância de azimute (°)",
        ),
        "Azimuth tolerance (degrees).",
    ),
    setting(
        "dip_tol",
        Kind::num(Some(0.0), None, 2.5),
        "/rs/vario/dip_tol",
        L::new(
            "Tolerancia inclinación (°)",
            "Dip tolerance (°)",
            "Tolerância de mergulho (°)",
        ),
        "Dip tolerance (degrees).",
    ),
    setting(
        "bw_h",
        Kind::num(Some(0.0), None, 5.0),
        "/rs/vario/bw_h",
        L::new(
            "Ancho de banda horizontal (0 = ninguno)",
            "Horizontal bandwidth (0 = none)",
            "Largura de banda horizontal (0 = nenhuma)",
        ),
        "Horizontal bandwidth (m).",
    ),
    setting(
        "bw_v",
        Kind::num(Some(0.0), None, 5.0),
        "/rs/vario/bw_v",
        L::new(
            "Ancho de banda vertical (0 = ninguno)",
            "Vertical bandwidth (0 = none)",
            "Largura de banda vertical (0 = nenhuma)",
        ),
        "Vertical bandwidth (m).",
    ),
    setting(
        "omni",
        Kind::Bool,
        "/rs/vario/omni",
        L::new(
            "Incluir omnidireccional",
            "Include omnidirectional",
            "Incluir omnidirecional",
        ),
        "Also compute the omnidirectional variogram.",
    ),
];

const RS_VARIOUPDATE: &[Param] = &[
    setting(
        "domain",
        Kind::Index {
            source: "domains",
            all: None,
        },
        "/rs/vario/domain",
        L::new("Dominio", "Domain", "Domínio"),
        "Estimation domain worked on (name or index).",
    ),
    setting(
        "nugget",
        Kind::num(Some(0.0), None, 0.5),
        "/rs/vario/work/nugget",
        L::new("Pepita", "Nugget", "Pepita"),
        "Nugget effect.",
    ),
    setting(
        "s1_kind",
        Kind::Choice {
            options: &[
                opt("spherical", L::new("Esférico", "Spherical", "Esférico")),
                opt("exponential", L::new("Exponencial", "Exponential", "Exponencial")),
                opt("gaussian", L::new("Gaussiano", "Gaussian", "Gaussiano")),
            ],
        },
        "/rs/vario/work/structures/0/kind",
        L::new("Tipo", "Type", "Tipo"),
        "Type of structure 1.",
    ),
    setting(
        "s1_sill",
        Kind::num(Some(0.0), None, 1.0),
        "/rs/vario/work/structures/0/sill",
        L::new(
            "Meseta parcial (0 = no usar)",
            "Partial sill (0 = unused)",
            "Patamar parcial (0 = não usar)",
        ),
        "Sill of structure 1 (0 = unused).",
    ),
    setting(
        "s1_r1",
        Kind::num(Some(0.1), None, 10.0),
        "/rs/vario/work/structures/0/ranges/0",
        L::new("Alcance eje mayor", "Major axis range", "Alcance eixo maior"),
        "Range of structure 1 along the major direction (m).",
    ),
    setting(
        "s1_r2",
        Kind::num(Some(0.1), None, 10.0),
        "/rs/vario/work/structures/0/ranges/1",
        L::new(
            "Alcance eje semi-mayor",
            "Semi-major axis range",
            "Alcance eixo semi-maior",
        ),
        "Range of structure 1 along the semi-major direction (m).",
    ),
    setting(
        "s1_r3",
        Kind::num(Some(0.1), None, 10.0),
        "/rs/vario/work/structures/0/ranges/2",
        L::new("Alcance eje menor", "Minor axis range", "Alcance eixo menor"),
        "Range of structure 1 along the minor direction (m).",
    ),
    setting(
        "s2_kind",
        Kind::Choice {
            options: &[
                opt("spherical", L::new("Esférico", "Spherical", "Esférico")),
                opt("exponential", L::new("Exponencial", "Exponential", "Exponencial")),
                opt("gaussian", L::new("Gaussiano", "Gaussian", "Gaussiano")),
            ],
        },
        "/rs/vario/work/structures/1/kind",
        L::new("Tipo", "Type", "Tipo"),
        "Type of structure 2.",
    ),
    setting(
        "s2_sill",
        Kind::num(Some(0.0), None, 1.0),
        "/rs/vario/work/structures/1/sill",
        L::new(
            "Meseta parcial (0 = no usar)",
            "Partial sill (0 = unused)",
            "Patamar parcial (0 = não usar)",
        ),
        "Sill of structure 2 (0 = unused).",
    ),
    setting(
        "s2_r1",
        Kind::num(Some(0.1), None, 10.0),
        "/rs/vario/work/structures/1/ranges/0",
        L::new("Alcance eje mayor", "Major axis range", "Alcance eixo maior"),
        "Range of structure 2 along the major direction (m).",
    ),
    setting(
        "s2_r2",
        Kind::num(Some(0.1), None, 10.0),
        "/rs/vario/work/structures/1/ranges/1",
        L::new(
            "Alcance eje semi-mayor",
            "Semi-major axis range",
            "Alcance eixo semi-maior",
        ),
        "Range of structure 2 along the semi-major direction (m).",
    ),
    setting(
        "s2_r3",
        Kind::num(Some(0.1), None, 10.0),
        "/rs/vario/work/structures/1/ranges/2",
        L::new("Alcance eje menor", "Minor axis range", "Alcance eixo menor"),
        "Range of structure 2 along the minor direction (m).",
    ),
];

const RS_FIT: &[Param] = &[
    setting(
        "domain",
        Kind::Index {
            source: "domains",
            all: None,
        },
        "/rs/vario/domain",
        L::new("Dominio", "Domain", "Domínio"),
        "Estimation domain worked on (name or index).",
    ),
    setting(
        "fit_kind",
        Kind::Choice {
            options: &[
                opt("spherical", L::new("Esférico", "Spherical", "Esférico")),
                opt("exponential", L::new("Exponencial", "Exponential", "Exponencial")),
                opt("gaussian", L::new("Gaussiano", "Gaussian", "Gaussiano")),
            ],
        },
        "/rs/vario/fit_kind1",
        L::new(
            "Tipo de la estructura 1 al ajustar",
            "Structure 1 type when fitting",
            "Tipo da estrutura 1 ao ajustar",
        ),
        "Structure type to fit.",
    ),
    setting(
        "fit_two",
        Kind::Bool,
        "/rs/vario/fit_two",
        L::new(
            "Ajustar dos estructuras",
            "Fit two structures",
            "Ajustar duas estruturas",
        ),
        "Fit two nested structures.",
    ),
    setting(
        "fix_nugget",
        Kind::Bool,
        "/rs/vario/fix_nugget",
        L::new("Mantener la pepita", "Keep the nugget", "Manter a pepita"),
        "Keep the current nugget.",
    ),
];

const RS_MODELSAVE: &[Param] = &[setting(
    "domain",
    Kind::Index {
        source: "domains",
        all: None,
    },
    "/rs/vario/domain",
    L::new("Dominio", "Domain", "Domínio"),
    "Estimation domain worked on (name or index).",
)];

const RS_MODELLOAD: &[Param] = &[setting(
    "domain",
    Kind::Index {
        source: "domains",
        all: None,
    },
    "/rs/vario/domain",
    L::new("Dominio", "Domain", "Domínio"),
    "Estimation domain worked on (name or index).",
)];

const RS_CROSSVAL: &[Param] = &[setting(
    "domain",
    Kind::Index {
        source: "domains",
        all: None,
    },
    "/rs/vario/domain",
    L::new("Dominio", "Domain", "Domínio"),
    "Estimation domain worked on (name or index).",
)];

const RS_BMCREATE: &[Param] = &[
    setting(
        "auto_extent",
        Kind::Bool,
        "/rs/bm/auto_extent",
        L::new(
            "Extensión automática (compósitos)",
            "Automatic extent (composites)",
            "Extensão automática (compósitos)",
        ),
        "Cover the composites automatically (origin and counts are then computed).",
    ),
    setting(
        "origin_x",
        Kind::num(None, None, 10.0),
        "/rs/bm/origin/0",
        L::new(
            "Origen (esquina inferior) X",
            "Origin (lower corner) X",
            "Origem (canto inferior) X",
        ),
        "Model origin X (corner, auto_extent off).",
    ),
    setting(
        "origin_y",
        Kind::num(None, None, 10.0),
        "/rs/bm/origin/1",
        L::new(
            "Origen (esquina inferior) Y",
            "Origin (lower corner) Y",
            "Origem (canto inferior) Y",
        ),
        "Model origin Y (corner, auto_extent off).",
    ),
    setting(
        "origin_z",
        Kind::num(None, None, 10.0),
        "/rs/bm/origin/2",
        L::new(
            "Origen (esquina inferior) Z",
            "Origin (lower corner) Z",
            "Origem (canto inferior) Z",
        ),
        "Model origin Z (corner, auto_extent off).",
    ),
    setting(
        "count_x",
        Kind::num(Some(1.0), None, 1.0),
        "/rs/bm/counts/0",
        L::new("Cantidad de bloques X", "Number of blocks X", "Quantidade de blocos X"),
        "Blocks along X (auto_extent off).",
    ),
    setting(
        "count_y",
        Kind::num(Some(1.0), None, 1.0),
        "/rs/bm/counts/1",
        L::new("Cantidad de bloques Y", "Number of blocks Y", "Quantidade de blocos Y"),
        "Blocks along Y (auto_extent off).",
    ),
    setting(
        "count_z",
        Kind::num(Some(1.0), None, 1.0),
        "/rs/bm/counts/2",
        L::new("Cantidad de bloques Z", "Number of blocks Z", "Quantidade de blocos Z"),
        "Blocks along Z (auto_extent off).",
    ),
    setting(
        "pad",
        Kind::num(Some(0.0), None, 10.0),
        "/rs/bm/pad",
        L::new("Margen (m)", "Margin (m)", "Margem (m)"),
        "Margin added around the composites (auto_extent, m).",
    ),
    setting(
        "size_x",
        Kind::num(Some(0.01), None, 1.0),
        "/rs/bm/size/0",
        L::new("Tamaño de bloque X", "Block size X", "Tamanho do bloco X"),
        "Block size along X (m).",
    ),
    setting(
        "size_y",
        Kind::num(Some(0.01), None, 1.0),
        "/rs/bm/size/1",
        L::new("Tamaño de bloque Y", "Block size Y", "Tamanho do bloco Y"),
        "Block size along Y (m).",
    ),
    setting(
        "size_z",
        Kind::num(Some(0.01), None, 1.0),
        "/rs/bm/size/2",
        L::new("Tamaño de bloque Z", "Block size Z", "Tamanho do bloco Z"),
        "Block size along Z (m).",
    ),
    setting(
        "rotation",
        Kind::num(None, None, 5.0),
        "/rs/bm/rotation",
        L::new("Rotación en planta (°)", "Plan rotation (°)", "Rotação em planta (°)"),
        "Rotation of the model in plan (degrees, clockwise).",
    ),
    setting(
        "disc_x",
        Kind::num(Some(1.0), None, 1.0),
        "/rs/bm/disc/0",
        L::new("Discretización X", "Discretisation X", "Discretização X"),
        "Block discretisation along X.",
    ),
    setting(
        "disc_y",
        Kind::num(Some(1.0), None, 1.0),
        "/rs/bm/disc/1",
        L::new("Discretización Y", "Discretisation Y", "Discretização Y"),
        "Block discretisation along Y.",
    ),
    setting(
        "disc_z",
        Kind::num(Some(1.0), None, 1.0),
        "/rs/bm/disc/2",
        L::new("Discretización Z", "Discretisation Z", "Discretização Z"),
        "Block discretisation along Z.",
    ),
    setting(
        "min_fraction",
        Kind::num(Some(0.0), None, 0.05),
        "/rs/bm/min_fraction",
        L::new(
            "Proporción mínima de bloque",
            "Minimum block proportion",
            "Proporção mínima do bloco",
        ),
        "Minimum fraction of a block inside its domain.",
    ),
];

const RS_ESTIMATE: &[Param] = &[
    setting(
        "auto_extent",
        Kind::Bool,
        "/rs/bm/auto_extent",
        L::new(
            "Extensión automática (compósitos)",
            "Automatic extent (composites)",
            "Extensão automática (compósitos)",
        ),
        "Cover the composites automatically (origin and counts are then computed).",
    ),
    setting(
        "origin_x",
        Kind::num(None, None, 10.0),
        "/rs/bm/origin/0",
        L::new(
            "Origen (esquina inferior) X",
            "Origin (lower corner) X",
            "Origem (canto inferior) X",
        ),
        "Model origin X (corner, auto_extent off).",
    ),
    setting(
        "origin_y",
        Kind::num(None, None, 10.0),
        "/rs/bm/origin/1",
        L::new(
            "Origen (esquina inferior) Y",
            "Origin (lower corner) Y",
            "Origem (canto inferior) Y",
        ),
        "Model origin Y (corner, auto_extent off).",
    ),
    setting(
        "origin_z",
        Kind::num(None, None, 10.0),
        "/rs/bm/origin/2",
        L::new(
            "Origen (esquina inferior) Z",
            "Origin (lower corner) Z",
            "Origem (canto inferior) Z",
        ),
        "Model origin Z (corner, auto_extent off).",
    ),
    setting(
        "count_x",
        Kind::num(Some(1.0), None, 1.0),
        "/rs/bm/counts/0",
        L::new("Cantidad de bloques X", "Number of blocks X", "Quantidade de blocos X"),
        "Blocks along X (auto_extent off).",
    ),
    setting(
        "count_y",
        Kind::num(Some(1.0), None, 1.0),
        "/rs/bm/counts/1",
        L::new("Cantidad de bloques Y", "Number of blocks Y", "Quantidade de blocos Y"),
        "Blocks along Y (auto_extent off).",
    ),
    setting(
        "count_z",
        Kind::num(Some(1.0), None, 1.0),
        "/rs/bm/counts/2",
        L::new("Cantidad de bloques Z", "Number of blocks Z", "Quantidade de blocos Z"),
        "Blocks along Z (auto_extent off).",
    ),
    setting(
        "pad",
        Kind::num(Some(0.0), None, 10.0),
        "/rs/bm/pad",
        L::new("Margen (m)", "Margin (m)", "Margem (m)"),
        "Margin added around the composites (auto_extent, m).",
    ),
    setting(
        "size_x",
        Kind::num(Some(0.01), None, 1.0),
        "/rs/bm/size/0",
        L::new("Tamaño de bloque X", "Block size X", "Tamanho do bloco X"),
        "Block size along X (m).",
    ),
    setting(
        "size_y",
        Kind::num(Some(0.01), None, 1.0),
        "/rs/bm/size/1",
        L::new("Tamaño de bloque Y", "Block size Y", "Tamanho do bloco Y"),
        "Block size along Y (m).",
    ),
    setting(
        "size_z",
        Kind::num(Some(0.01), None, 1.0),
        "/rs/bm/size/2",
        L::new("Tamaño de bloque Z", "Block size Z", "Tamanho do bloco Z"),
        "Block size along Z (m).",
    ),
    setting(
        "rotation",
        Kind::num(None, None, 5.0),
        "/rs/bm/rotation",
        L::new("Rotación en planta (°)", "Plan rotation (°)", "Rotação em planta (°)"),
        "Rotation of the model in plan (degrees, clockwise).",
    ),
    setting(
        "disc_x",
        Kind::num(Some(1.0), None, 1.0),
        "/rs/bm/disc/0",
        L::new("Discretización X", "Discretisation X", "Discretização X"),
        "Block discretisation along X.",
    ),
    setting(
        "disc_y",
        Kind::num(Some(1.0), None, 1.0),
        "/rs/bm/disc/1",
        L::new("Discretización Y", "Discretisation Y", "Discretização Y"),
        "Block discretisation along Y.",
    ),
    setting(
        "disc_z",
        Kind::num(Some(1.0), None, 1.0),
        "/rs/bm/disc/2",
        L::new("Discretización Z", "Discretisation Z", "Discretização Z"),
        "Block discretisation along Z.",
    ),
    setting(
        "min_fraction",
        Kind::num(Some(0.0), None, 0.05),
        "/rs/bm/min_fraction",
        L::new(
            "Proporción mínima de bloque",
            "Minimum block proportion",
            "Proporção mínima do bloco",
        ),
        "Minimum fraction of a block inside its domain.",
    ),
    setting(
        "ordinary",
        Kind::Bool,
        "/rs/bm/ordinary",
        L::new("Krigeado ordinario", "Ordinary kriging", "Krigagem ordinária"),
        "Ordinary kriging.",
    ),
    setting(
        "simple",
        Kind::Bool,
        "/rs/bm/simple",
        L::new("Krigeado simple", "Simple kriging", "Krigagem simples"),
        "Simple kriging (declustered mean).",
    ),
    setting(
        "idw",
        Kind::Bool,
        "/rs/bm/idw",
        L::new("Inverso de la distancia", "Inverse distance", "Inverso da distância"),
        "Inverse distance.",
    ),
    setting(
        "idw_power",
        Kind::num(Some(0.1), None, 0.5),
        "/rs/bm/idw_power",
        L::new("Potencia IDW", "IDW power", "Potência IDW"),
        "Inverse distance power.",
    ),
    setting(
        "nearest",
        Kind::Bool,
        "/rs/bm/nearest",
        L::new("Vecino más cercano", "Nearest neighbour", "Vizinho mais próximo"),
        "Nearest neighbour.",
    ),
    setting(
        "radius1",
        Kind::num(Some(0.1), None, 10.0),
        "/rs/bm/radii/0",
        L::new(
            "Radios del elipsoide (mayor, semi, menor) 1",
            "Ellipsoid radii (major, semi, minor) 1",
            "Raios do elipsoide (maior, semi, menor) 1",
        ),
        "Search radius 1 (major, semi-major, minor; oriented like the variogram model, m).",
    ),
    setting(
        "radius2",
        Kind::num(Some(0.1), None, 10.0),
        "/rs/bm/radii/1",
        L::new(
            "Radios del elipsoide (mayor, semi, menor) 2",
            "Ellipsoid radii (major, semi, minor) 2",
            "Raios do elipsoide (maior, semi, menor) 2",
        ),
        "Search radius 2 (major, semi-major, minor; oriented like the variogram model, m).",
    ),
    setting(
        "radius3",
        Kind::num(Some(0.1), None, 10.0),
        "/rs/bm/radii/2",
        L::new(
            "Radios del elipsoide (mayor, semi, menor) 3",
            "Ellipsoid radii (major, semi, minor) 3",
            "Raios do elipsoide (maior, semi, menor) 3",
        ),
        "Search radius 3 (major, semi-major, minor; oriented like the variogram model, m).",
    ),
    setting(
        "min_samples",
        Kind::num(Some(1.0), None, 1.0),
        "/rs/bm/min_samples",
        L::new("Mínimo de compósitos", "Minimum composites", "Mínimo de compósitos"),
        "Minimum composites per block (pass 1).",
    ),
    setting(
        "max_samples",
        Kind::num(Some(1.0), None, 1.0),
        "/rs/bm/max_samples",
        L::new("Máximo de compósitos", "Maximum composites", "Máximo de compósitos"),
        "Maximum composites per block.",
    ),
    setting(
        "max_per_octant",
        Kind::num(Some(0.0), None, 1.0),
        "/rs/bm/max_per_octant",
        L::new(
            "Máximo por octante (0 = sin límite)",
            "Maximum per octant (0 = unlimited)",
            "Máximo por octante (0 = sem limite)",
        ),
        "Maximum composites per octant (0 = no limit).",
    ),
    setting(
        "max_per_hole",
        Kind::num(Some(0.0), None, 1.0),
        "/rs/bm/max_per_hole",
        L::new(
            "Máximo por sondaje (0 = sin límite)",
            "Maximum per hole (0 = unlimited)",
            "Máximo por furo (0 = sem limite)",
        ),
        "Maximum composites per hole (0 = no limit).",
    ),
    setting(
        "pass2",
        Kind::num(Some(0.0), None, 0.5),
        "/rs/bm/pass2",
        L::new(
            "Pasada 2: multiplicador de radios (0 = no)",
            "Pass 2: radius multiplier (0 = off)",
            "Passada 2: multiplicador de raios (0 = não)",
        ),
        "Radius multiplier of search pass 2 (<= 1 = no pass).",
    ),
    setting(
        "pass2_min",
        Kind::num(Some(1.0), None, 1.0),
        "/rs/bm/pass2_min",
        L::new("Pasada 2: mínimo", "Pass 2: minimum", "Passada 2: mínimo"),
        "Minimum composites of pass 2.",
    ),
    setting(
        "pass3",
        Kind::num(Some(0.0), None, 0.5),
        "/rs/bm/pass3",
        L::new(
            "Pasada 3: multiplicador de radios (0 = no)",
            "Pass 3: radius multiplier (0 = off)",
            "Passada 3: multiplicador de raios (0 = não)",
        ),
        "Radius multiplier of search pass 3 (<= 1 = no pass).",
    ),
    setting(
        "pass3_min",
        Kind::num(Some(1.0), None, 1.0),
        "/rs/bm/pass3_min",
        L::new("Pasada 3: mínimo", "Pass 3: minimum", "Passada 3: mínimo"),
        "Minimum composites of pass 3.",
    ),
];

const RS_VALIDATE: &[Param] = &[
    setting(
        "chart",
        Kind::Choice {
            options: &[
                opt(
                    "swath",
                    L::new(
                        "Swath: modelo vs compósitos",
                        "Swath: model vs composites",
                        "Swath: modelo vs compósitos",
                    ),
                ),
                opt(
                    "grade_tonnage",
                    L::new("Curva tonelaje-ley", "Grade-tonnage curve", "Curva tonelagem-teor"),
                ),
                opt(
                    "histogram",
                    L::new(
                        "Histograma de compósitos",
                        "Composite histogram",
                        "Histograma de compósitos",
                    ),
                ),
            ],
        },
        "/rs/bm/chart",
        L::new("Gráfico", "Chart", "Gráfico"),
        "Chart to show.",
    ),
    setting(
        "swath_axis",
        Kind::Choice {
            options: &[
                opt("x", L::new("Este (X)", "Easting (X)", "Leste (X)")),
                opt("y", L::new("Norte (Y)", "Northing (Y)", "Norte (Y)")),
                opt("z", L::new("Cota (Z)", "Elevation (Z)", "Cota (Z)")),
            ],
        },
        "/rs/bm/swath_axis",
        L::new("Eje del swath", "Swath axis", "Eixo do swath"),
        "Direction of the swath plot.",
    ),
    setting(
        "swath_width",
        Kind::num(Some(0.1), None, 10.0),
        "/rs/bm/swath_width",
        L::new("Ancho del swath (m)", "Swath width (m)", "Largura do swath (m)"),
        "Width of the swaths (m).",
    ),
];

const RS_BMVIEW: &[Param] = &[
    setting(
        "mode",
        Kind::Choice {
            options: &[
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
                        "Cubos sobre la ley de corte",
                        "Cubes above the cut-off",
                        "Cubos acima do teor de corte",
                    ),
                ),
            ],
        },
        "/rs/bm/view_mode",
        L::new("Vista", "View", "Vista"),
        "View type.",
    ),
    setting(
        "attribute",
        Kind::Choice {
            options: &[
                opt("grade_ok", L::new("Ley (OK)", "Grade (OK)", "Teor (OK)")),
                opt("ok_variance", L::new("Varianza OK", "OK variance", "Variância OK")),
                opt(
                    "kriging_efficiency",
                    L::new("Eficiencia de krigeado", "Kriging efficiency", "Eficiência da krigagem"),
                ),
                opt("domain", L::new("Dominio", "Domain", "Domínio")),
                opt("pass", L::new("Pasada de búsqueda", "Search pass", "Passada de busca")),
                opt(
                    "samples",
                    L::new("N° de compósitos", "Number of composites", "N° de compósitos"),
                ),
                opt(
                    "distance",
                    L::new("Distancia media", "Mean distance", "Distância média"),
                ),
                opt(
                    "nn",
                    L::new("Vecino más cercano", "Nearest neighbour", "Vizinho mais próximo"),
                ),
                opt("idw", L::new("IDW", "IDW", "IDW")),
                opt("sk", L::new("Krigeado simple", "Simple kriging", "Krigagem simples")),
                opt("tonnage", L::new("Tonelaje", "Tonnage", "Tonelagem")),
                opt(
                    "gt_rmr",
                    L::new("RMR (geotecnia)", "RMR (geotechnics)", "RMR (geotecnia)"),
                ),
                opt(
                    "gt_gsi",
                    L::new("GSI (geotecnia)", "GSI (geotechnics)", "GSI (geotecnia)"),
                ),
                opt(
                    "gt_logq",
                    L::new("log Q (geotecnia)", "log Q (geotechnics)", "log Q (geotecnia)"),
                ),
                opt(
                    "gt_class",
                    L::new(
                        "Clase de macizo (geotecnia)",
                        "Rock mass class (geotechnics)",
                        "Classe do maciço (geotecnia)",
                    ),
                ),
                opt(
                    "gt_dist",
                    L::new(
                        "Distancia media a los registros (geotecnia)",
                        "Mean distance to records (geotechnics)",
                        "Distância média aos registros (geotecnia)",
                    ),
                ),
                opt(
                    "gt_n",
                    L::new(
                        "N° de registros (geotecnia)",
                        "Number of records (geotechnics)",
                        "N.º de registros (geotecnia)",
                    ),
                ),
            ],
        },
        "/rs/bm/view_attr",
        L::new("Colorear por", "Colour by", "Colorir por"),
        "Column that colours the blocks.",
    ),
    setting(
        "cutoff",
        Kind::num(None, None, 0.5),
        "/rs/bm/view_cutoff",
        L::new("Ley de corte", "Cut-off grade", "Teor de corte"),
        "Grade cut-off of the grade attributes.",
    ),
    setting(
        "domain",
        Kind::Index {
            source: "domains",
            all: Some(L::new("Todos", "All", "Todos")),
        },
        "/rs/bm/view_domain",
        L::new("Dominio", "Domain", "Domínio"),
        "Domain shown (name or index; 0 = all).",
    ),
    setting(
        "z_from",
        Kind::num(None, None, 12.0),
        "/rs/bm/view_z_from",
        L::new("Cota inicial", "Start elevation", "Cota inicial"),
        "Lowest level (mode levels; from = to = every block level).",
    ),
    setting(
        "z_to",
        Kind::num(None, None, 12.0),
        "/rs/bm/view_z_to",
        L::new(
            "Cota final (≤ inicial = automática)",
            "End elevation (≤ start = automatic)",
            "Cota final (≤ inicial = automática)",
        ),
        "Highest level (mode levels).",
    ),
    setting(
        "z_step",
        Kind::num(Some(0.1), None, 1.0),
        "/rs/bm/view_z_step",
        L::new("Intervalo de cota", "Elevation interval", "Intervalo de cota"),
        "Level spacing (mode levels).",
    ),
    setting(
        "spacing",
        Kind::num(Some(0.1), None, 5.0),
        "/rs/bm/view_spacing",
        L::new(
            "Espaciado de secciones (m)",
            "Section spacing (m)",
            "Espaçamento das seções (m)",
        ),
        "Spacing of parallel sections (mode two_points).",
    ),
    setting(
        "count",
        Kind::num(Some(1.0), None, 1.0),
        "/rs/bm/view_count",
        L::new("Cantidad de secciones", "Number of sections", "Quantidade de seções"),
        "Number of parallel sections (mode two_points).",
    ),
    setting(
        "interval",
        Kind::num(Some(0.1), None, 5.0),
        "/rs/bm/view_interval",
        L::new(
            "Intervalo a lo largo del eje (m)",
            "Interval along the axis (m)",
            "Intervalo ao longo do eixo (m)",
        ),
        "Section spacing along the axis (mode axis).",
    ),
    setting(
        "limit",
        Kind::num(Some(1.0), None, 1000.0),
        "/rs/bm/view_limit",
        L::new("Máximo de polígonos", "Maximum polygons", "Máximo de polígonos"),
        "Maximum polygons / cubes drawn.",
    ),
    input(
        "axis",
        Kind::Handle,
        false,
        L::new("Eje", "Axis", "Eixo"),
        "Axis polyline (mode axis). Without it a pick is started.",
    ),
    input(
        "p1",
        Kind::Point,
        false,
        L::new("Punto 1", "Point 1", "Ponto 1"),
        "First point of the section line (mode two_points).",
    ),
    input(
        "p2",
        Kind::Point,
        false,
        L::new("Punto 2", "Point 2", "Ponto 2"),
        "Second point of the section line.",
    ),
];

const RS_BMLOAD: &[Param] = &[setting(
    "folder",
    Kind::Text,
    "/rs/bm/folder",
    L::new(
        "Carpeta del modelo (vacía = junto al dibujo)",
        "Model folder (empty = next to the drawing)",
        "Pasta do modelo (vazia = junto ao desenho)",
    ),
    "Folder of the model files (empty = beside the drawing).",
)];

const RS_BMEXPORT: &[Param] = &[setting(
    "folder",
    Kind::Text,
    "/rs/bm/folder",
    L::new(
        "Carpeta del modelo (vacía = junto al dibujo)",
        "Model folder (empty = next to the drawing)",
        "Pasta do modelo (vazia = junto ao desenho)",
    ),
    "Folder of the model files (empty = beside the drawing).",
)];

pub static OPS: &[OpDef] = &[
OpDef { name: "rs_composite", command: "MIN_RS_COMPOSITE", aliases: &[],
            label: L::new("Compositar y analizar", "Composite and analyse", "Compositar e analisar"), group: Group::Resources, kind: OpKind::Action,
            summary: "Composite the assays of dh_assay_attr by domain (domains table: geology values, solids, surface conditions), cap them, decluster, report statistics and the histogram. Saves <drawing>_composites.csv. Needs dh_load.", params: RS_COMPOSITE,
            positional: &[], result: "`variable`, `composites`, `domains` (per domain: `name`, `count`, `mean`, `declustered_mean`, `cv`, `min`, `max`), `file`." },
OpDef { name: "rs_drawcomp", command: "MIN_RS_DRAWCOMP", aliases: &[],
            label: L::new("Dibujar compósitos", "Draw composites", "Desenhar compósitos"), group: Group::Resources, kind: OpKind::Action,
            summary: "Draw the composites coloured by grade.", params: RS_DRAWCOMP,
            positional: &[], result: "`composites` (drawn)." },
OpDef { name: "rs_setsolid", command: "MIN_RS_SETSOLID", aliases: &[],
            label: L::new("Asignar sólidos seleccionados", "Assign selected solids", "Atribuir sólidos selecionados"), group: Group::Resources, kind: OpKind::Action,
            summary: "Assign solids (closed meshes) to an estimation domain (stored in their XDATA OPENMINING_DOM).", params: RS_SETSOLID,
            positional: &[], result: "`domain`, `solids` (assigned)." },
OpDef { name: "rs_contact", command: "MIN_RS_CONTACT", aliases: &[],
            label: L::new("Calcular superficie de contacto", "Compute contact surface", "Calcular superfície de contato"), group: Group::Resources, kind: OpKind::Action,
            summary: "Krige the contact surfaces from the geology contacts in the holes (contacts table) and draw them.", params: RS_CONTACT,
            positional: &[], result: "`surfaces` (per surface: `name`, `points`, `nodes`)." },
OpDef { name: "rs_vario", command: "MIN_RS_VARIO", aliases: &[],
            label: L::new("Calcular variogramas", "Compute variograms", "Calcular variogramas"), group: Group::Resources, kind: OpKind::Action,
            summary: "Experimental variograms of the domain's composites along the major, semi-major and minor directions (and omnidirectional); shows the chart. Needs rs_composite.", params: RS_VARIO,
            positional: &[], result: "`domain`, `pairs` (per direction), `directions`." },
OpDef { name: "rs_varioupdate", command: "MIN_RS_VARIOUPDATE", aliases: &[],
            label: L::new("Actualizar gráfico", "Refresh chart", "Atualizar gráfico"), group: Group::Resources, kind: OpKind::Action,
            summary: "Redraw the variogram chart with the model typed (nugget, two nested structures).", params: RS_VARIOUPDATE,
            positional: &[], result: "`domain`, `model` (nugget, structures)." },
OpDef { name: "rs_fit", command: "MIN_RS_FIT", aliases: &[],
            label: L::new("Autoajustar", "Auto-fit", "Ajuste automático"), group: Group::Resources, kind: OpKind::Action,
            summary: "Fit the variogram model to the experimental variograms (weighted least squares, Nelder-Mead) and put it in the model fields.", params: RS_FIT,
            positional: &[], result: "`domain`, `model` (nugget, structures), `error`." },
OpDef { name: "rs_modelsave", command: "MIN_RS_MODELSAVE", aliases: &[],
            label: L::new("Guardar en el dominio", "Save to domain", "Salvar no domínio"), group: Group::Resources, kind: OpKind::Action,
            summary: "Store the working variogram model as the model of the domain (used by rs_estimate).", params: RS_MODELSAVE,
            positional: &[], result: "`domain`." },
OpDef { name: "rs_modelload", command: "MIN_RS_MODELLOAD", aliases: &[],
            label: L::new("Cargar del dominio", "Load from domain", "Carregar do domínio"), group: Group::Resources, kind: OpKind::Action,
            summary: "Load the stored model of the domain into the working model fields.", params: RS_MODELLOAD,
            positional: &[], result: "`domain`, `model`." },
OpDef { name: "rs_crossval", command: "MIN_RS_CROSSVAL", aliases: &[],
            label: L::new("Validación cruzada", "Cross-validation", "Validação cruzada"), group: Group::Resources, kind: OpKind::Action,
            summary: "Leave-one-out cross-validation of the domain's model with the search settings.", params: RS_CROSSVAL,
            positional: &[], result: "`domain`, `n`, `mean_error`, `mse`, `correlation`, `slope`." },
OpDef { name: "rs_bmcreate", command: "MIN_RS_BMCREATE", aliases: &[],
            label: L::new("Crear modelo", "Create model", "Criar modelo"), group: Group::Resources, kind: OpKind::Action,
            summary: "Create the block model (geometry settings) and assign each block to its domain (geology, solids, surfaces). Needs rs_composite.", params: RS_BMCREATE,
            positional: &[], result: "`blocks`, `counts` (`[nx,ny,nz]`), `assigned` (blocks in a domain)." },
OpDef { name: "rs_estimate", command: "MIN_RS_ESTIMATE", aliases: &[],
            label: L::new("Estimar", "Estimate", "Estimar"), group: Group::Resources, kind: OpKind::Action,
            summary: "Create the block model and estimate each domain with its saved variogram model (OK, SK, IDW, NN; search passes); reports resources by domain at the view cut-off and saves <drawing>.ombm. Needs rs_composite and rs_modelsave for every domain.", params: RS_ESTIMATE,
            positional: &[], result: "`blocks`, `estimated`, `seconds`, `domains` (per domain: `name`, `blocks`, `estimated`, `per_pass`, `samples`), `resources` (per domain: `domain`, `tonnes`, `grade`, `blocks`), `saved`." },
OpDef { name: "rs_validate", command: "MIN_RS_VALIDATE", aliases: &[],
            label: L::new("Validar", "Validate", "Validar"), group: Group::Resources, kind: OpKind::Action,
            summary: "Validate the model against the composites: swath plot, grade-tonnage curves or histogram (chart in the panel) and global statistics.", params: RS_VALIDATE,
            positional: &[], result: "`lines` (validation statistics as text)." },
OpDef { name: "rs_bmview", command: "MIN_RS_BMVIEW", aliases: &[],
            label: L::new("Dibujar vista", "Draw view", "Desenhar vista"), group: Group::Resources, kind: OpKind::Action,
            summary: "Draw the block model: level plans, sections through 2 points or along an axis, or cubes; coloured by an attribute (grade, variance, domain, pass, geotechnical columns...).", params: RS_BMVIEW,
            positional: &[], result: "`planes`, `polygons`, `limited`, `cubes` (`[drawn,total]` in cubes mode)." },
OpDef { name: "rs_bmload", command: "MIN_RS_BMLOAD", aliases: &[],
            label: L::new("Cargar archivo", "Load file", "Carregar arquivo"), group: Group::Resources, kind: OpKind::Action,
            summary: "Load the block model saved beside the drawing (<drawing>.ombm).", params: RS_BMLOAD,
            positional: &[], result: "`blocks`, `file`." },
OpDef { name: "rs_bmexport", command: "MIN_RS_BMEXPORT", aliases: &[],
            label: L::new("Exportar CSV", "Export CSV", "Exportar CSV"), group: Group::Resources, kind: OpKind::Action,
            summary: "Export the block model to <drawing>_bloques.csv.", params: RS_BMEXPORT,
            positional: &[], result: "`file`." },
];

pub static PANELS: &[PanelSpec] = &[
PanelSpec { id: "min_rs_comp", title: L::new("Compósitos y dominios", "Composites and domains", "Compósitos e domínios"), command: "MIN_RS_COMP_PANEL", items: &[Item::Section { id: "rs_f_comp", title: L::new("Compósitos", "Composites", "Compósitos"), items: &[Item::Param("rs_composite", "comp_length"), Item::Param("rs_composite", "comp_remainder"), Item::Param("rs_composite", "comp_min_fraction"), Item::Param("rs_composite", "comp_min_coverage"), Item::Param("rs_composite", "unit")], collapsed: false }, Item::Section { id: "rs_f_dom", title: L::new("Dominios", "Domains", "Domínios"), items: &[Item::Param("rs_composite", "domains"), Item::Info(L::new("Columna Superficie: «Encima C1», «Debajo C1», o dos condiciones separadas por ; para quedar entre dos contactos («Debajo C1; Encima C2»). Sin nombre se usa la primera superficie.", "Surface column: «Above C1», «Below C1», or two conditions separated by ; to lie between two contacts («Below C1; Above C2»). Without a name the first surface is used.", "Coluna Superfície: «Acima C1», «Abaixo C1», ou duas condições separadas por ; para ficar entre dois contatos («Abaixo C1; Acima C2»). Sem nome usa-se a primeira superfície."))], collapsed: false }, Item::Section { id: "rs_f_dc", title: L::new("Desagrupamiento", "Declustering", "Desagrupamento"), items: &[Item::Param("rs_composite", "dc_min"), Item::Param("rs_composite", "dc_max"), Item::Param("rs_composite", "dc_steps"), Item::Param("rs_composite", "dc_zratio")], collapsed: false }, Item::Button("rs_composite"), Item::Status("rs_comp_status"), Item::Chart("rs_comp_chart"), Item::Row(&[Item::Button("rs_drawcomp")]), Item::Section { id: "rs_f_solid", title: L::new("Dominio por sólido", "Domain by solid", "Domínio por sólido"), items: &[Item::Param("rs_setsolid", "target"), Item::Button("rs_setsolid")], collapsed: false }, Item::Section { id: "rs_f_ct", title: L::new("Superficie de contacto", "Contact surface", "Superfície de contato"), items: &[Item::Param("rs_contact", "contacts"), Item::Param("rs_contact", "step"), Item::Param("rs_contact", "kind"), Item::Param("rs_contact", "nugget"), Item::Param("rs_contact", "sill"), Item::Param("rs_contact", "range"), Item::Param("rs_contact", "radius"), Item::Param("rs_contact", "min_samples"), Item::Param("rs_contact", "max_samples"), Item::Button("rs_contact")], collapsed: false }] },
PanelSpec { id: "min_rs_vario", title: L::new("Variografía", "Variography", "Variografia"), command: "MIN_RS_VARIO_PANEL", items: &[Item::Param("rs_vario", "domain"), Item::Section { id: "rs_f_vexp", title: L::new("Variograma experimental", "Experimental variogram", "Variograma experimental"), items: &[Item::Param("rs_vario", "azimuth"), Item::Param("rs_vario", "dip"), Item::Param("rs_vario", "rake"), Item::Param("rs_vario", "lag"), Item::Param("rs_vario", "nlags"), Item::Param("rs_vario", "az_tol"), Item::Param("rs_vario", "dip_tol"), Item::Param("rs_vario", "bw_h"), Item::Param("rs_vario", "bw_v"), Item::Param("rs_vario", "omni"), Item::Button("rs_vario")], collapsed: false }, Item::Status("rs_v_status"), Item::Chart("rs_v_chart"), Item::Section { id: "rs_f_vmodel", title: L::new("Modelo teórico", "Theoretical model", "Modelo teórico"), items: &[Item::Param("rs_varioupdate", "nugget"), Item::Section { id: "rs_f_s1", title: L::new("Estructura 1", "Structure 1", "Estrutura 1"), items: &[Item::Param("rs_varioupdate", "s1_kind"), Item::Param("rs_varioupdate", "s1_sill"), Item::Param("rs_varioupdate", "s1_r1"), Item::Param("rs_varioupdate", "s1_r2"), Item::Param("rs_varioupdate", "s1_r3")], collapsed: false }, Item::Section { id: "rs_f_s2", title: L::new("Estructura 2", "Structure 2", "Estrutura 2"), items: &[Item::Param("rs_varioupdate", "s2_kind"), Item::Param("rs_varioupdate", "s2_sill"), Item::Param("rs_varioupdate", "s2_r1"), Item::Param("rs_varioupdate", "s2_r2"), Item::Param("rs_varioupdate", "s2_r3")], collapsed: false }], collapsed: false }, Item::Row(&[Item::Button("rs_varioupdate")]), Item::Section { id: "rs_f_vfit", title: L::new("Autoajuste", "Auto-fit", "Ajuste automático"), items: &[Item::Param("rs_fit", "fit_kind"), Item::Param("rs_fit", "fit_two"), Item::Param("rs_fit", "fix_nugget"), Item::Button("rs_fit")], collapsed: false }, Item::Row(&[Item::Button("rs_modelsave"), Item::Button("rs_modelload")]), Item::Button("rs_crossval")] },
PanelSpec { id: "min_rs_bm", title: L::new("Modelo de bloques", "Block model", "Modelo de blocos"), command: "MIN_RS_BM_PANEL", items: &[Item::Section { id: "rs_f_geo", title: L::new("Geometría", "Geometry", "Geometria"), items: &[Item::Param("rs_bmcreate", "auto_extent"), Item::Param("rs_bmcreate", "origin_x"), Item::Param("rs_bmcreate", "origin_y"), Item::Param("rs_bmcreate", "origin_z"), Item::Param("rs_bmcreate", "count_x"), Item::Param("rs_bmcreate", "count_y"), Item::Param("rs_bmcreate", "count_z"), Item::Param("rs_bmcreate", "pad"), Item::Param("rs_bmcreate", "size_x"), Item::Param("rs_bmcreate", "size_y"), Item::Param("rs_bmcreate", "size_z"), Item::Param("rs_bmcreate", "rotation"), Item::Param("rs_bmcreate", "disc_x"), Item::Param("rs_bmcreate", "disc_y"), Item::Param("rs_bmcreate", "disc_z")], collapsed: false }, Item::Section { id: "rs_f_meth", title: L::new("Métodos", "Methods", "Métodos"), items: &[Item::Param("rs_estimate", "ordinary"), Item::Param("rs_estimate", "simple"), Item::Param("rs_estimate", "idw"), Item::Param("rs_estimate", "idw_power"), Item::Param("rs_estimate", "nearest")], collapsed: false }, Item::Section { id: "rs_f_search", title: L::new("Búsqueda", "Search", "Busca"), items: &[Item::Param("rs_estimate", "radius1"), Item::Param("rs_estimate", "radius2"), Item::Param("rs_estimate", "radius3"), Item::Param("rs_estimate", "min_samples"), Item::Param("rs_estimate", "max_samples"), Item::Param("rs_estimate", "max_per_octant"), Item::Param("rs_estimate", "max_per_hole"), Item::Param("rs_estimate", "pass2"), Item::Param("rs_estimate", "pass2_min"), Item::Param("rs_estimate", "pass3"), Item::Param("rs_estimate", "pass3_min"), Item::Param("rs_bmcreate", "min_fraction")], collapsed: false }, Item::Row(&[Item::Button("rs_bmcreate"), Item::Button("rs_estimate")]), Item::Status("rs_b_status"), Item::Section { id: "rs_f_chart", title: L::new("Validación", "Validation", "Validação"), items: &[Item::Param("rs_validate", "chart"), Item::Param("rs_validate", "swath_axis"), Item::Param("rs_validate", "swath_width"), Item::Button("rs_validate"), Item::Chart("rs_b_chart")], collapsed: false }, Item::Section { id: "rs_f_view", title: L::new("Vista", "View", "Vista"), items: &[Item::Param("rs_bmview", "mode"), Item::Param("rs_bmview", "attribute"), Item::Param("rs_bmview", "cutoff"), Item::Param("rs_bmview", "domain"), Item::Param("rs_bmview", "z_from"), Item::Param("rs_bmview", "z_to"), Item::Param("rs_bmview", "z_step"), Item::Param("rs_bmview", "spacing"), Item::Param("rs_bmview", "count"), Item::Param("rs_bmview", "interval"), Item::Param("rs_bmview", "limit"), Item::Button("rs_bmview")], collapsed: false }, Item::Section { id: "rs_f_store", title: L::new("Archivo", "File", "Arquivo"), items: &[Item::Param("rs_bmload", "folder"), Item::Row(&[Item::Button("rs_bmload"), Item::Button("rs_bmexport")])], collapsed: false }] },
];
