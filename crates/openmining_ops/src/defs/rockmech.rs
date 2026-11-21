//! Geotechnics phases 3 and 4: structures (stereonet, sets, kinematics,
//! wedges) and stresses and design (in situ stress, Kirsch,
//! convergence-confinement, pillars, crown pillars).

use crate::{input, opt, setting, Group, Item, Kind, OpDef, OpKind, Opt, PanelSpec, Param, L};

const DEG: Kind = Kind::num(Some(0.0), Some(90.0), 1.0);
const AZ: Kind = Kind::num(Some(0.0), Some(360.0), 5.0);

const STEREO: &[Param] = &[
    setting(
        "joints",
        Kind::Bool,
        "/rm/src_joints",
        L::new(
            "Tabla de discontinuidades",
            "Discontinuity table",
            "Tabela de descontinuidades",
        ),
        "Use the oriented discontinuities loaded with gt_load.",
    ),
    setting(
        "traces",
        Kind::Bool,
        "/rm/src_traces",
        L::new(
            "Trazas geológicas con buzamiento",
            "Geological traces with dip",
            "Traços geológicos com mergulho",
        ),
        "Also use the geological traces (gm_trace) that store a dip and dip direction.",
    ),
    setting(
        "contours",
        Kind::Bool,
        "/rm/contours",
        L::new("Contornos de densidad", "Density contours", "Contornos de densidade"),
        "Draw pole density contours (1 % counting cone).",
    ),
];

const SETS: &[Param] = &[
    setting("n_sets", Kind::Integer { min: Some(0), max: Some(6) }, "/rm/n_sets", L::new("Número de familias (0 = automático)", "Number of sets (0 = automatic)", "Número de famílias (0 = automático)"), "Sets to find; 0 picks the density peaks that hold at least 3 poles (at most 6)."),
    setting("min_angle", Kind::num(Some(5.0), Some(90.0), 5.0), "/rm/min_angle", L::new("Ángulo mínimo entre familias (°)", "Minimum angle between sets (°)", "Ângulo mínimo entre famílias (°)"), "Minimum angle between set mean poles."),
    setting("max_angle", Kind::num(Some(5.0), Some(90.0), 5.0), "/rm/max_angle", L::new("Ángulo máximo a la familia (°)", "Maximum angle to the set (°)", "Ângulo máximo à família (°)"), "Poles farther than this from every set stay unassigned."),
    setting(
        "sets",
        Kind::Table {
            columns: &[
                L::new("Familia", "Set", "Família"),
                L::new("Buzamiento", "Dip", "Mergulho"),
                L::new("Dir. buzamiento", "Dip direction", "Dir. mergulho"),
                L::new("φ (°)", "φ (°)", "φ (°)"),
                L::new("c (kPa)", "c (kPa)", "c (kPa)"),
            ],
            codec: "joint_sets",
        },
        "/rm/sets",
        L::new("Familias", "Sets", "Famílias"),
        "Discontinuity sets: rows of [name, dip, dip direction, friction degrees, cohesion kPa]. Filled by st_sets, editable.",
    ),
];

const KIN: &[Param] = &[
    setting(
        "face_dip",
        DEG,
        "/rm/face_dip",
        L::new("Buzamiento de la cara", "Face dip", "Mergulho da face"),
        "Dip of the slope or excavation face (90 = vertical wall).",
    ),
    setting(
        "face_dipdir",
        AZ,
        "/rm/face_dipdir",
        L::new(
            "Dirección de buzamiento de la cara",
            "Face dip direction",
            "Direção de mergulho da face",
        ),
        "Dip direction of the face (direction it faces).",
    ),
    setting(
        "friction",
        DEG,
        "/rm/friction",
        L::new("Fricción φ (°)", "Friction φ (°)", "Atrito φ (°)"),
        "Friction angle of the discontinuities.",
    ),
    setting(
        "lateral",
        Kind::num(Some(0.0), Some(90.0), 5.0),
        "/rm/lateral",
        L::new("Límites laterales (°)", "Lateral limits (°)", "Limites laterais (°)"),
        "Lateral limits for planar sliding and toppling (usually 20).",
    ),
];

const WEDGES: &[Param] = &[
    setting("axis_trend", AZ, "/rm/axis_trend", L::new("Rumbo del eje (°)", "Axis trend (°)", "Rumo do eixo (°)"), "Trend of the working's axis (ignored when an axis polyline is given)."),
    setting("axis_plunge", Kind::num(Some(-90.0), Some(90.0), 1.0), "/rm/axis_plunge", L::new("Inclinación del eje (°)", "Axis plunge (°)", "Inclinação do eixo (°)"), "Plunge of the axis (positive down)."),
    setting("width", Kind::num(Some(0.1), None, 0.5), "/rm/width", L::new("Ancho de la labor (m)", "Opening width (m)", "Largura da escavação (m)"), "Width of the opening (span of roof wedges)."),
    setting("height", Kind::num(Some(0.1), None, 0.5), "/rm/height", L::new("Alto de la labor (m)", "Opening height (m)", "Altura da escavação (m)"), "Height of the opening (span of wall wedges)."),
    setting("max_length", Kind::num(Some(0.0), None, 1.0), "/rm/max_length", L::new("Largo máximo de cuña (m, 0 = sin límite)", "Maximum wedge length (m, 0 = no limit)", "Comprimento máximo da cunha (m, 0 = sem limite)"), "Limit of the wedge trace along the axis (e.g. joint trace length)."),
    setting("set_a", Kind::Integer { min: Some(1), max: Some(20) }, "/rm/set_a", L::new("Familia A", "Set A", "Família A"), "First set of the wedge (1-based row of the sets table)."),
    setting("set_b", Kind::Integer { min: Some(1), max: Some(20) }, "/rm/set_b", L::new("Familia B", "Set B", "Família B"), "Second set (row)."),
    setting("set_c", Kind::Integer { min: Some(1), max: Some(20) }, "/rm/set_c", L::new("Familia C", "Set C", "Família C"), "Third set (row)."),
    setting("gamma", Kind::num(Some(1.0), None, 1.0), "/rm/gamma", L::new("Peso unitario (kN/m³)", "Unit weight (kN/m³)", "Peso específico (kN/m³)"), "Unit weight of the rock (wedges, in situ stress)."),
    setting("support", Kind::num(Some(0.0), None, 10.0), "/rm/support", L::new("Presión de soporte (kPa)", "Support pressure (kPa)", "Pressão de suporte (kPa)"), "Support pressure on the wedge base (bolts, shotcrete); 0 = none."),
    setting("chainage", Kind::num(Some(0.0), None, 5.0), "/rm/chainage", L::new("Progresiva para dibujar (m)", "Chainage to draw at (m)", "Estaca para desenhar (m)"), "Horizontal chainage on the axis polyline where the wedges are drawn."),
    input("axis", Kind::Handle, false, L::new("Eje", "Axis", "Eixo"), "Axis polyline: its direction at `chainage` replaces the trend/plunge and the wedges are drawn there."),
    input("at", Kind::Point, false, L::new("Punto", "Point", "Ponto"), "Draw the wedges with the axis passing through this point (floor level per the workings settings). Without axis or at: computed only."),
];

const ST_DRAW: &[Param] = &[
    setting(
        "radius",
        Kind::num(Some(0.1), None, 10.0),
        "/rm/draw_radius",
        L::new(
            "Radio del estereograma (m)",
            "Stereonet radius (m)",
            "Raio do estereograma (m)",
        ),
        "Radius of the stereonet drawn in the DWG.",
    ),
    input(
        "at",
        Kind::Point,
        false,
        L::new("Centro", "Centre", "Centro"),
        "Centre of the stereonet (world x,y[,z]; default 0,0,0).",
    ),
];

const STRESS: &[Param] = &[
    setting(
        "depth",
        Kind::num(Some(1.0), None, 50.0),
        "/rm/depth",
        L::new("Profundidad (m)", "Depth (m)", "Profundidade (m)"),
        "Depth below surface of the working or pillars.",
    ),
    setting(
        "gamma",
        Kind::num(Some(1.0), None, 1.0),
        "/rm/gamma",
        L::new(
            "Peso unitario (kN/m³)",
            "Unit weight (kN/m³)",
            "Peso específico (kN/m³)",
        ),
        "Unit weight of the overburden.",
    ),
    setting(
        "k_h",
        Kind::num(Some(0.0), None, 0.1),
        "/rm/k_h",
        L::new("K = σH / σv", "K = σH / σv", "K = σH / σv"),
        "Ratio of the major horizontal to the vertical stress.",
    ),
    setting(
        "k_hmin",
        Kind::num(Some(0.0), None, 0.1),
        "/rm/k_hmin",
        L::new("K = σh / σv", "K = σh / σv", "K = σh / σv"),
        "Ratio of the minor horizontal to the vertical stress.",
    ),
    setting(
        "az_h",
        AZ,
        "/rm/az_h",
        L::new("Azimut de σH (°)", "Azimuth of σH (°)", "Azimute de σH (°)"),
        "Azimuth of the major horizontal stress.",
    ),
    setting(
        "axis_trend",
        AZ,
        "/rm/axis_trend",
        L::new("Rumbo del eje (°)", "Axis trend (°)", "Rumo do eixo (°)"),
        "Trend of the working whose section stresses are computed.",
    ),
];

const OPENINGS: &[Opt] = &[
    opt(
        "circle",
        L::new("Circular (radio)", "Circular (radius)", "Circular (raio)"),
    ),
    opt(
        "equivalent",
        L::new(
            "Radio equivalente (ancho × alto)",
            "Equivalent radius (width × height)",
            "Raio equivalente (largura × altura)",
        ),
    ),
];

const KIRSCH: &[Param] = &[
    setting(
        "opening",
        Kind::Choice { options: OPENINGS },
        "/rm/opening",
        L::new("Abertura", "Opening", "Abertura"),
        "Circular opening of `radius`, or the circle with the area of width x height (approximation).",
    ),
    setting(
        "radius",
        Kind::num(Some(0.1), None, 0.5),
        "/rm/radius",
        L::new("Radio (m)", "Radius (m)", "Raio (m)"),
        "Radius of the circular opening.",
    ),
];

const CONV: &[Param] = &[
    setting(
        "e_mass",
        Kind::num(Some(0.0), None, 500.0),
        "/rm/e_mass",
        L::new(
            "Módulo del macizo (MPa, 0 = Hoek-Diederichs)",
            "Rock mass modulus (MPa, 0 = Hoek-Diederichs)",
            "Módulo do maciço (MPa, 0 = Hoek-Diederichs)",
        ),
        "Rock mass modulus; 0 takes it from the Hoek-Brown panel.",
    ),
    setting(
        "nu",
        Kind::num(Some(0.0), Some(0.49), 0.05),
        "/rm/nu",
        L::new("Poisson del macizo", "Rock mass Poisson ratio", "Poisson do maciço"),
        "Poisson's ratio of the rock mass.",
    ),
    setting(
        "install",
        Kind::num(Some(0.0), None, 0.5),
        "/rm/install",
        L::new(
            "Distancia al frente al instalar (m)",
            "Distance to the face at installation (m)",
            "Distância à frente na instalação (m)",
        ),
        "Distance behind the face where the support is installed.",
    ),
    setting(
        "shotcrete",
        Kind::Bool,
        "/rm/use_shotcrete",
        L::new("Shotcrete", "Shotcrete", "Concreto projetado"),
        "Include a shotcrete lining.",
    ),
    setting(
        "sc_thickness",
        Kind::num(Some(0.01), None, 0.05),
        "/rm/sc_t",
        L::new("Espesor (m)", "Thickness (m)", "Espessura (m)"),
        "Shotcrete thickness.",
    ),
    setting(
        "sc_strength",
        Kind::num(Some(1.0), None, 5.0),
        "/rm/sc_sigma",
        L::new("Resistencia σcc (MPa)", "Strength σcc (MPa)", "Resistência σcc (MPa)"),
        "Shotcrete uniaxial compressive strength.",
    ),
    setting(
        "sc_modulus",
        Kind::num(Some(100.0), None, 1000.0),
        "/rm/sc_e",
        L::new("Módulo (MPa)", "Modulus (MPa)", "Módulo (MPa)"),
        "Shotcrete Young's modulus.",
    ),
    setting(
        "bolts",
        Kind::Bool,
        "/rm/use_bolts",
        L::new("Pernos", "Rock bolts", "Tirantes"),
        "Include mechanically anchored rock bolts.",
    ),
    setting(
        "b_length",
        Kind::num(Some(0.1), None, 0.5),
        "/rm/b_length",
        L::new("Largo del perno (m)", "Bolt length (m)", "Comprimento do tirante (m)"),
        "Bolt length.",
    ),
    setting(
        "b_diameter",
        Kind::num(Some(0.001), None, 0.002),
        "/rm/b_diam",
        L::new("Diámetro (m)", "Diameter (m)", "Diâmetro (m)"),
        "Bolt diameter.",
    ),
    setting(
        "b_spacing_c",
        Kind::num(Some(0.1), None, 0.25),
        "/rm/b_sc",
        L::new(
            "Espaciado circunferencial (m)",
            "Circumferential spacing (m)",
            "Espaçamento circunferencial (m)",
        ),
        "Bolt spacing around the tunnel.",
    ),
    setting(
        "b_spacing_l",
        Kind::num(Some(0.1), None, 0.25),
        "/rm/b_sl",
        L::new(
            "Espaciado longitudinal (m)",
            "Longitudinal spacing (m)",
            "Espaçamento longitudinal (m)",
        ),
        "Bolt spacing along the tunnel.",
    ),
    setting(
        "b_capacity",
        Kind::num(Some(0.001), None, 0.01),
        "/rm/b_tbf",
        L::new(
            "Capacidad del perno (MN)",
            "Bolt capacity (MN)",
            "Capacidade do tirante (MN)",
        ),
        "Ultimate load of one bolt (MN); use the bolt's value.",
    ),
    setting(
        "b_q",
        Kind::num(Some(0.0), None, 0.01),
        "/rm/b_q",
        L::new(
            "Constante carga-deformación Q (m/MN)",
            "Deformation-load constant Q (m/MN)",
            "Constante carga-deformação Q (m/MN)",
        ),
        "Deformation-load constant of the anchor and plate (m/MN), from pull tests.",
    ),
];

const SHAPES: &[Opt] = &[
    opt("square", L::new("Cuadrado", "Square", "Quadrado")),
    opt("rectangular", L::new("Rectangular", "Rectangular", "Retangular")),
    opt("rib", L::new("Corrido (rib)", "Rib", "Corrido (rib)")),
];

const FORMULAS: &[Opt] = &[
    opt("lunder_pakalnis", L::same("Lunder-Pakalnis (1997)")),
    opt("obert_duvall", L::same("Obert-Duvall (1967)")),
    opt("bieniawski", L::same("Bieniawski (1975)")),
    opt("hedley_grant", L::same("Hedley-Grant (1972)")),
];

const PILLAR: &[Param] = &[
    setting(
        "shape",
        Kind::Choice { options: SHAPES },
        "/rm/pl_shape",
        L::new("Forma", "Shape", "Forma"),
        "Pillar shape for the tributary area.",
    ),
    setting(
        "width",
        Kind::num(Some(0.1), None, 0.5),
        "/rm/pl_width",
        L::new("Ancho del pilar (m)", "Pillar width (m)", "Largura do pilar (m)"),
        "Pillar width.",
    ),
    setting(
        "length",
        Kind::num(Some(0.1), None, 0.5),
        "/rm/pl_length",
        L::new("Largo del pilar (m)", "Pillar length (m)", "Comprimento do pilar (m)"),
        "Pillar length (rectangular).",
    ),
    setting(
        "height",
        Kind::num(Some(0.1), None, 0.5),
        "/rm/pl_height",
        L::new("Alto del pilar (m)", "Pillar height (m)", "Altura do pilar (m)"),
        "Pillar height.",
    ),
    setting(
        "room",
        Kind::num(Some(0.1), None, 0.5),
        "/rm/pl_room",
        L::new("Ancho de cámara (m)", "Room width (m)", "Largura da câmara (m)"),
        "Width of the rooms between pillars.",
    ),
    setting(
        "ucs",
        Kind::num(Some(0.0), None, 10.0),
        "/rm/pl_ucs",
        L::new(
            "UCS roca intacta (MPa, 0 = Hoek-Brown)",
            "Intact UCS (MPa, 0 = Hoek-Brown)",
            "UCS rocha intacta (MPa, 0 = Hoek-Brown)",
        ),
        "Intact rock UCS for the strength formula; 0 takes σci of the Hoek-Brown panel.",
    ),
    setting(
        "formula",
        Kind::Choice { options: FORMULAS },
        "/rm/pl_formula",
        L::new("Fórmula de resistencia", "Strength formula", "Fórmula de resistência"),
        "Empirical pillar strength formula.",
    ),
];

const SPAN: &[Param] = &[
    setting("span", Kind::num(Some(0.1), None, 1.0), "/rm/sp_span", L::new("Luz S (m)", "Span S (m)", "Vão S (m)"), "Span of the crown pillar."),
    setting("strike_length", Kind::num(Some(0.1), None, 10.0), "/rm/sp_length", L::new("Largo en el rumbo L (m)", "Strike length L (m)", "Comprimento no rumo L (m)"), "Length along strike."),
    setting("thickness", Kind::num(Some(0.1), None, 1.0), "/rm/sp_thickness", L::new("Espesor t (m)", "Thickness t (m)", "Espessura t (m)"), "Thickness of the crown pillar."),
    setting("gravity", Kind::num(Some(0.1), None, 0.1), "/rm/sp_gravity", L::new("Peso específico (t/m³)", "Specific gravity (t/m³)", "Peso específico (t/m³)"), "Specific gravity of the crown pillar rock."),
    setting("dip", DEG, "/rm/sp_dip", L::new("Buzamiento del cuerpo (°)", "Orebody dip (°)", "Mergulho do corpo (°)"), "Dip of the orebody / stope walls."),
    setting("critical", Kind::num(Some(0.0), None, 0.5), "/rm/sp_critical", L::new("Luz escalada crítica Sc (0 = ninguna)", "Critical scaled span Sc (0 = none)", "Vão escalado crítico Sc (0 = nenhum)"), "Critical scaled span from your reference (e.g. Carter's Sc-Q relation); compared with Cs. Not computed by the plugin."),
];

const CHARTS: &[Opt] = &[
    opt(
        "kirsch",
        L::new(
            "Kirsch (σθ en el contorno)",
            "Kirsch (boundary σθ)",
            "Kirsch (σθ no contorno)",
        ),
    ),
    opt(
        "convergence",
        L::new(
            "Convergencia-confinamiento",
            "Convergence-confinement",
            "Convergência-confinamento",
        ),
    ),
];

const SM_DRAW: &[Param] = &[
    setting(
        "chart",
        Kind::Choice { options: CHARTS },
        "/rm/draw_chart",
        L::new("Gráfico", "Chart", "Gráfico"),
        "Which computed chart to draw.",
    ),
    setting(
        "size",
        Kind::num(Some(1.0), None, 10.0),
        "/rm/draw_size",
        L::new("Ancho del gráfico (m)", "Chart width (m)", "Largura do gráfico (m)"),
        "Width of the drawn chart.",
    ),
    input(
        "at",
        Kind::Point,
        false,
        L::new("Esquina", "Corner", "Canto"),
        "Lower-left corner of the chart (world; default 0,0,0).",
    ),
];

const fn op(
    name: &'static str,
    command: &'static str,
    label: L,
    summary: &'static str,
    params: &'static [Param],
    result: &'static str,
) -> OpDef {
    OpDef {
        name,
        command,
        aliases: &[],
        label,
        group: Group::Geotechnics,
        kind: OpKind::Action,
        summary,
        params,
        positional: &[],
        result,
    }
}

pub static OPS: &[OpDef] = &[
    op("st_stereonet", "MIN_ST_STEREONET", L::new("Estereograma", "Stereonet", "Estereograma"),
        "Equal-area lower-hemisphere stereonet of the discontinuity poles (table and/or traces with dip), density contours and the set planes; chart in the panel.",
        STEREO, "`poles`, `from_table`, `from_traces`, `max_density` (% per 1 %), `sets` (shown)."),
    op("st_sets", "MIN_ST_SETS", L::new("Calcular familias", "Find sets", "Calcular famílias"),
        "Group the poles into discontinuity sets (spherical clustering; Fisher K and 95 % cone) and write them into the sets table (friction and cohesion of existing rows are kept).",
        SETS, "`sets`: list of `{name, dip, dipdir, count, k, alpha95}`, `unassigned`."),
    op("st_kinematic", "MIN_ST_KINEMATIC", L::new("Análisis cinemático", "Kinematic analysis", "Análise cinemática"),
        "Markland kinematic analysis for a slope or excavation face: planar sliding, wedge sliding and flexural toppling (direct toppling not included), for every pole, pole pair and set.",
        KIN, "`planar`, `wedge`, `flexural` (`{critical, of, pct}`), `planar_sets`, `wedge_sets` (pairs of set names), `flexural_sets`."),
    op("st_wedges", "MIN_ST_WEDGES", L::new("Cuñas", "Wedges", "Cunhas"),
        "Largest wedges of three sets in the roof and walls of a working (planar faces), with volume, weight, failure mode and factor of safety (Hoek-Bray vector method, support pressure optional); drawn when an axis or point is given.",
        WEDGES, "`sets` (names used), `wedges`: list of `{face, mode, fs, volume, weight, apex_height, base_area}` (fs null = stable); `drawn`."),
    op("st_draw", "MIN_ST_DRAW", L::new("Dibujar estereograma", "Draw stereonet", "Desenhar estereograma"),
        "Draw the stereonet (net, poles, contours, set great circles) in the DWG as 2D entities.",
        ST_DRAW, "`entities`."),
    op("sm_stress", "MIN_SM_STRESS", L::new("Esfuerzos in situ", "In situ stress", "Tensões in situ"),
        "In situ stresses from the depth gradient (sigma_v = gamma H, sigma_H, sigma_h by K ratios) and their components in the section of a working with the given trend.",
        STRESS, "`sigma_v`, `sigma_h_max`, `sigma_h_min`, `section`: `{sigma_x, sigma_z, tau_xz, sigma_1, sigma_3, angle, sigma_axial}` (MPa)."),
    op("sm_kirsch", "MIN_SM_KIRSCH", L::new("Kirsch", "Kirsch", "Kirsch"),
        "Elastic stresses around a circular opening (Kirsch) in the section stress field: boundary tangential stress, strength factor with the Hoek-Brown rock mass and depth of the overstressed zone; chart in the panel. Non-circular openings use an equivalent radius (approximation).",
        KIRSCH, "`radius`, `equivalent`, `p1`, `p3`, `sigma_t_max`, `sigma_t_min`, `sigma_cm`, `strength_factor`, `overstressed_depth` (m)."),
    op("sm_convergence", "MIN_SM_CONVERGENCE", L::new("Convergencia-confinamiento", "Convergence-confinement", "Convergência-confinamento"),
        "Convergence-confinement of the working: Duncan Fama ground curve (c', phi' of the Hoek-Brown fit), Vlachopoulos-Diederichs displacement profile, shotcrete and bolt support curves, equilibrium, support factor of safety and Hoek-Marinos squeezing class; chart in the panel.",
        CONV, "`radius`, `p0`, `c`, `phi`, `e_mass`, `p_cr`, `plastic_radius`, `u_max` (m), `strain` (%), `squeezing`, `u_install`, `p_eq`, `u_eq`, `support_fs`, `support_p_max`."),
    op("sm_pillar", "MIN_SM_PILLAR", L::new("Pilares", "Pillars", "Pilares"),
        "Pillar stress by tributary area (vertical stress of sm_stress), extraction ratio, strength by the chosen empirical formula (all formulas reported) and factor of safety.",
        PILLAR, "`load`, `extraction`, `width_height`, `strength`, `fs`, `formula`, `all` (formula -> `{strength, fs}`)."),
    op("sm_span", "MIN_SM_SPAN", L::new("Pilar corona", "Crown pillar", "Pilar coroa"),
        "Scaled span of a crown pillar (Carter 1992), compared with the critical scaled span you give.",
        SPAN, "`scaled_span`, `critical` (null if not given), `ratio` (Sc / Cs)."),
    op("sm_draw", "MIN_SM_DRAW", L::new("Dibujar gráfico", "Draw chart", "Desenhar gráfico"),
        "Draw the last Kirsch or convergence-confinement chart in the DWG (frame, axes, curves).",
        SM_DRAW, "`entities`."),
];

const STRUCT_ITEMS: &[Item] = &[
    Item::Section {
        id: "rm_f_src",
        title: L::new("Datos", "Data", "Dados"),
        items: &[
            Item::Param("st_stereonet", "joints"),
            Item::Param("st_stereonet", "traces"),
            Item::Param("st_stereonet", "contours"),
            Item::Button("st_stereonet"),
        ],
        collapsed: false,
    },
    Item::Section {
        id: "rm_f_sets",
        title: L::new("Familias", "Sets", "Famílias"),
        items: &[
            Item::Param("st_sets", "n_sets"),
            Item::Param("st_sets", "min_angle"),
            Item::Param("st_sets", "max_angle"),
            Item::Button("st_sets"),
            Item::Param("st_sets", "sets"),
        ],
        collapsed: false,
    },
    Item::Section {
        id: "rm_f_kin",
        title: L::new("Cinemática", "Kinematics", "Cinemática"),
        items: &[
            Item::Param("st_kinematic", "face_dip"),
            Item::Param("st_kinematic", "face_dipdir"),
            Item::Param("st_kinematic", "friction"),
            Item::Param("st_kinematic", "lateral"),
            Item::Button("st_kinematic"),
        ],
        collapsed: false,
    },
    Item::Status("rm_st_status"),
    Item::Chart("rm_st_chart"),
    Item::Section {
        id: "rm_f_wedge",
        title: L::new("Cuñas en labores", "Wedges in workings", "Cunhas em escavações"),
        items: &[
            Item::Param("st_wedges", "axis_trend"),
            Item::Param("st_wedges", "axis_plunge"),
            Item::Param("st_wedges", "width"),
            Item::Param("st_wedges", "height"),
            Item::Param("st_wedges", "max_length"),
            Item::Param("st_wedges", "set_a"),
            Item::Param("st_wedges", "set_b"),
            Item::Param("st_wedges", "set_c"),
            Item::Param("st_wedges", "gamma"),
            Item::Param("st_wedges", "support"),
            Item::Param("st_wedges", "chainage"),
            Item::Button("st_wedges"),
        ],
        collapsed: false,
    },
    Item::Section {
        id: "rm_f_stdraw",
        title: L::new("Dibujo", "Drawing", "Desenho"),
        items: &[Item::Param("st_draw", "radius"), Item::Button("st_draw")],
        collapsed: true,
    },
];

const DESIGN_ITEMS: &[Item] = &[
    Item::Section {
        id: "rm_f_stress",
        title: L::new("Esfuerzos in situ", "In situ stress", "Tensões in situ"),
        items: &[
            Item::Param("sm_stress", "depth"),
            Item::Param("sm_stress", "gamma"),
            Item::Param("sm_stress", "k_h"),
            Item::Param("sm_stress", "k_hmin"),
            Item::Param("sm_stress", "az_h"),
            Item::Param("sm_stress", "axis_trend"),
            Item::Button("sm_stress"),
        ],
        collapsed: false,
    },
    Item::Section {
        id: "rm_f_kirsch",
        title: L::new("Esfuerzos alrededor de la labor", "Stresses around the opening", "Tensões ao redor da escavação"),
        items: &[Item::Param("sm_kirsch", "opening"), Item::Param("sm_kirsch", "radius"), Item::Button("sm_kirsch")],
        collapsed: false,
    },
    Item::Section {
        id: "rm_f_conv",
        title: L::new("Convergencia-confinamiento", "Convergence-confinement", "Convergência-confinamento"),
        items: &[
            Item::Param("sm_convergence", "e_mass"),
            Item::Param("sm_convergence", "nu"),
            Item::Param("sm_convergence", "install"),
            Item::Param("sm_convergence", "shotcrete"),
            Item::Param("sm_convergence", "sc_thickness"),
            Item::Param("sm_convergence", "sc_strength"),
            Item::Param("sm_convergence", "sc_modulus"),
            Item::Param("sm_convergence", "bolts"),
            Item::Param("sm_convergence", "b_length"),
            Item::Param("sm_convergence", "b_diameter"),
            Item::Param("sm_convergence", "b_spacing_c"),
            Item::Param("sm_convergence", "b_spacing_l"),
            Item::Param("sm_convergence", "b_capacity"),
            Item::Param("sm_convergence", "b_q"),
            Item::Button("sm_convergence"),
        ],
        collapsed: false,
    },
    Item::Status("rm_sm_status"),
    Item::Chart("rm_sm_chart"),
    Item::Row(&[Item::Param("sm_draw", "chart"), Item::Param("sm_draw", "size")]),
    Item::Button("sm_draw"),
    Item::Section {
        id: "rm_f_pillar",
        title: L::new("Pilares", "Pillars", "Pilares"),
        items: &[
            Item::Param("sm_pillar", "shape"),
            Item::Param("sm_pillar", "width"),
            Item::Param("sm_pillar", "length"),
            Item::Param("sm_pillar", "height"),
            Item::Param("sm_pillar", "room"),
            Item::Param("sm_pillar", "ucs"),
            Item::Param("sm_pillar", "formula"),
            Item::Button("sm_pillar"),
        ],
        collapsed: false,
    },
    Item::Section {
        id: "rm_f_span",
        title: L::new("Pilar corona", "Crown pillar", "Pilar coroa"),
        items: &[
            Item::Param("sm_span", "span"),
            Item::Param("sm_span", "strike_length"),
            Item::Param("sm_span", "thickness"),
            Item::Param("sm_span", "gravity"),
            Item::Param("sm_span", "dip"),
            Item::Param("sm_span", "critical"),
            Item::Info(L::new(
                "La luz escalada crítica (relación Sc–Q o gráfico de Lang) es empírica: indíquela desde su referencia.",
                "The critical scaled span (Sc-Q relation or Lang's chart) is empirical: give it from your reference.",
                "O vão escalado crítico (relação Sc–Q ou gráfico de Lang) é empírico: indique-o a partir da sua referência.",
            )),
            Item::Button("sm_span"),
        ],
        collapsed: false,
    },
];

pub static PANELS: &[PanelSpec] = &[
    PanelSpec {
        id: "min_rm_struct",
        title: L::new(
            "Geotecnia: estructuras",
            "Geotechnics: structures",
            "Geotecnia: estruturas",
        ),
        command: "MIN_RM_STRUCT_PANEL",
        items: STRUCT_ITEMS,
    },
    PanelSpec {
        id: "min_rm_design",
        title: L::new(
            "Geotecnia: esfuerzos y diseño",
            "Geotechnics: stresses and design",
            "Geotecnia: tensões e projeto",
        ),
        command: "MIN_RM_DESIGN_PANEL",
        items: DESIGN_ITEMS,
    },
];
