//! Underground workings: solids along an axis or from boundaries.

#[allow(unused_imports)]
use crate::{input, opt, setting, Group, Item, Kind, OpDef, OpKind, PanelSpec, Param, L, TABLE_FILES};

#[allow(dead_code)]
const FILES: Kind = Kind::Path {
    extensions: TABLE_FILES,
};

const UG_AXIS: &[Param] = &[setting("profile", Kind::Choice { options: &[opt("rectangular", L::new("Rectangular", "Rectangular", "Retangular")), opt("trapezoidal", L::new("Trapezoidal", "Trapezoidal", "Trapezoidal")), opt("circular", L::new("Circular", "Circular", "Circular")), opt("horseshoe", L::new("Herradura", "Horseshoe", "Ferradura")), opt("custom", L::new("Personalizado (polilínea)", "Custom (polyline)", "Personalizado (polilinha)"))] }, "/ug/profile", L::new("Perfil", "Profile", "Perfil"), "Section shape; custom takes a closed polyline (profile_polyline)."), setting("width", Kind::num(Some(0.01), None, 0.5), "/ug/width", L::new("Ancho / base inferior", "Width / bottom", "Largura / base inferior"), "Section width (m)."), setting("height", Kind::num(Some(0.01), None, 0.5), "/ug/height", L::new("Alto", "Height", "Altura"), "Section height (m)."), setting("top_width", Kind::num(Some(0.01), None, 0.5), "/ug/top_width", L::new("Base superior", "Top width", "Base superior"), "Top width of a trapezoidal section (m)."), setting("radius", Kind::num(Some(0.01), None, 0.25), "/ug/radius", L::new("Radio", "Radius", "Raio"), "Radius of a circular section or of the horseshoe crown (m)."), setting("wall_height", Kind::num(Some(0.01), None, 0.5), "/ug/wall_height", L::new("Altura de paredes", "Wall height", "Altura das paredes"), "Wall height of a horseshoe section (m)."), setting("h_align", Kind::Choice { options: &[opt("left", L::new("Izquierda", "Left", "Esquerda")), opt("centre", L::new("Centro", "Centre", "Centro")), opt("right", L::new("Derecha", "Right", "Direita"))] }, "/ug/h_align", L::new("Alineación horizontal", "Horizontal alignment", "Alinhamento horizontal"), "Horizontal position of the axis on the section."), setting("v_align", Kind::Choice { options: &[opt("top", L::new("Superior", "Top", "Superior")), opt("centre", L::new("Centro", "Centre", "Centro")), opt("bottom", L::new("Inferior", "Bottom", "Inferior"))] }, "/ug/v_align", L::new("Alineación vertical", "Vertical alignment", "Alinhamento vertical"), "Vertical position of the axis on the section (bottom = axis on the floor)."), setting("interval", Kind::num(Some(0.01), None, 0.5), "/ug/interval", L::new("Intervalo de secciones", "Section interval", "Intervalo de seções"), "Distance between sections along the axis (m)."), setting("draw_sections", Kind::Bool, "/ug/draw_sections", L::new("Dibujar secciones", "Draw sections", "Desenhar seções"), "Also draw the sections as closed polylines."), setting("tangent_3d", Kind::Bool, "/ug/tangent_3d", L::new("Tangente 3D (rampas)", "3D tangent (ramps)", "Tangente 3D (rampas)"), "Sections perpendicular to the 3D tangent (inclined axes) instead of vertical."), input("axis", Kind::Handle, false, L::new("Eje", "Axis", "Eixo"), "3D polyline / polyline / line of the axis. Without it: the single selected entity, or a pick (status waiting_input)."), input("profile_polyline", Kind::Handle, false, L::new("Perfil", "Profile", "Perfil"), "Closed polyline of the custom section (profile = custom), drawn in plan with the axis at 0,0.")];

const UG_HBOUND: &[Param] = &[
    setting(
        "interval",
        Kind::num(Some(0.01), None, 0.5),
        "/ug/interval",
        L::new("Intervalo de secciones", "Section interval", "Intervalo de seções"),
        "Distance between sections along the axis (m).",
    ),
    setting(
        "draw_sections",
        Kind::Bool,
        "/ug/draw_sections",
        L::new("Dibujar secciones", "Draw sections", "Desenhar seções"),
        "Also draw the sections as closed polylines.",
    ),
    setting(
        "tangent_3d",
        Kind::Bool,
        "/ug/tangent_3d",
        L::new("Tangente 3D (rampas)", "3D tangent (ramps)", "Tangente 3D (rampas)"),
        "Sections perpendicular to the 3D tangent (inclined axes) instead of vertical.",
    ),
    input(
        "lines",
        Kind::Handles,
        false,
        L::new("Contornos", "Boundaries", "Contornos"),
        "Four polylines in this order: roof, floor, left, right. Without them a pick is started.",
    ),
];

const UG_VBOUND: &[Param] = &[
    setting(
        "interval",
        Kind::num(Some(0.01), None, 0.5),
        "/ug/interval",
        L::new("Intervalo de secciones", "Section interval", "Intervalo de seções"),
        "Distance between sections along the axis (m).",
    ),
    setting(
        "draw_sections",
        Kind::Bool,
        "/ug/draw_sections",
        L::new("Dibujar secciones", "Draw sections", "Desenhar seções"),
        "Also draw the sections as closed polylines.",
    ),
    setting(
        "tangent_3d",
        Kind::Bool,
        "/ug/tangent_3d",
        L::new("Tangente 3D (rampas)", "3D tangent (ramps)", "Tangente 3D (rampas)"),
        "Sections perpendicular to the 3D tangent (inclined axes) instead of vertical.",
    ),
    input(
        "lines",
        Kind::Handles,
        false,
        L::new("Contornos", "Boundaries", "Contornos"),
        "Four polylines in this order: front, back, left, right. Without them a pick is started.",
    ),
];

pub static OPS: &[OpDef] = &[
    OpDef {
        name: "ug_axis",
        command: "MIN_UG_AXIS",
        aliases: &[],
        label: L::new("Por eje", "By axis", "Por eixo"),
        group: Group::Workings,
        kind: OpKind::Action,
        summary: "Closed solid (mesh) of a working along an axis polyline with the configured section.",
        params: UG_AXIS,
        positional: &[],
        result: "`volume` (m3), `sections`, `solid` (id).",
    },
    OpDef {
        name: "ug_hbound",
        command: "MIN_UG_HBOUND",
        aliases: &[],
        label: L::new("Contornos H", "H boundaries", "Contornos H"),
        group: Group::Workings,
        kind: OpKind::Action,
        summary: "Closed solid of a working from four boundary polylines: roof, floor, left, right.",
        params: UG_HBOUND,
        positional: &[],
        result: "`volume` (m3), `sections`, `solid`.",
    },
    OpDef {
        name: "ug_vbound",
        command: "MIN_UG_VBOUND",
        aliases: &[],
        label: L::new("Contornos V", "V boundaries", "Contornos V"),
        group: Group::Workings,
        kind: OpKind::Action,
        summary: "Closed solid of a working from four boundary polylines: front, back, left, right.",
        params: UG_VBOUND,
        positional: &[],
        result: "`volume` (m3), `sections`, `solid`.",
    },
];

pub static PANELS: &[PanelSpec] = &[PanelSpec {
    id: "min_workings",
    title: L::new(
        "Labores subterráneas",
        "Underground workings",
        "Escavações subterrâneas",
    ),
    command: "MIN_UG_PANEL",
    items: &[
        Item::Section {
            id: "ug_section",
            title: L::new("Sección", "Section", "Seção"),
            items: &[
                Item::Param("ug_axis", "profile"),
                Item::Param("ug_axis", "width"),
                Item::Param("ug_axis", "height"),
                Item::Param("ug_axis", "top_width"),
                Item::Param("ug_axis", "radius"),
                Item::Param("ug_axis", "wall_height"),
                Item::Param("ug_axis", "h_align"),
                Item::Param("ug_axis", "v_align"),
            ],
            collapsed: false,
        },
        Item::Param("ug_axis", "interval"),
        Item::Param("ug_axis", "draw_sections"),
        Item::Param("ug_axis", "tangent_3d"),
        Item::Button("ug_axis"),
        Item::Row(&[Item::Button("ug_hbound"), Item::Button("ug_vbound")]),
    ],
}];
