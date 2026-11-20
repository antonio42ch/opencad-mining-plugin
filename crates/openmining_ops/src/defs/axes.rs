//! Axis designer: straights and curves with grade, drawn in series.

#[allow(unused_imports)]
use crate::{input, opt, setting, Group, Item, Kind, OpDef, OpKind, PanelSpec, Param, L, TABLE_FILES};

#[allow(dead_code)]
const FILES: Kind = Kind::Path {
    extensions: TABLE_FILES,
};

const AX_LINE: &[Param] = &[setting("e", Kind::num(None, None, 1.0), "/ax/e", L::new("Este", "Easting", "Leste"), "Start point east (world coordinates)."), setting("n", Kind::num(None, None, 1.0), "/ax/n", L::new("Norte", "Northing", "Norte"), "Start point north."), setting("z", Kind::num(None, None, 1.0), "/ax/z", L::new("Cota", "Elevation", "Cota"), "Start point elevation."), setting("bearing", Kind::Text, "/ax/bearing", L::new("Rumbo / azimut", "Bearing / azimuth", "Rumo / azimute"), "Start direction: azimuth in degrees or bearing (N45E, S30W, N10O)."), setting("grade", Kind::num(None, None, 0.5), "/ax/grade", L::new("Pendiente (+ sube)", "Grade (+ up)", "Inclinação (+ sobe)"), "Grade of the element (positive up)."), setting("grade_unit", Kind::Choice { options: &[opt("percent", L::new("%", "%", "%")), opt("degrees", L::new("°", "°", "°"))] }, "/ax/grade_unit", L::new("Unidad", "Unit", "Unidade"), "Unit of the grade."), setting("update_start", Kind::Bool, "/ax/update_start", L::new("Actualizar punto de inicio", "Update start point", "Atualizar ponto inicial"), "Continue the current axis: the element is appended and the start moves to its end. Off = each element is a new axis from the start point."), setting("length", Kind::num(Some(0.001), None, 1.0), "/ax/length", L::new("Longitud horizontal", "Horizontal length", "Comprimento horizontal"), "Horizontal length of the straight (m).")];

const AX_CURVE: &[Param] = &[setting("e", Kind::num(None, None, 1.0), "/ax/e", L::new("Este", "Easting", "Leste"), "Start point east (world coordinates)."), setting("n", Kind::num(None, None, 1.0), "/ax/n", L::new("Norte", "Northing", "Norte"), "Start point north."), setting("z", Kind::num(None, None, 1.0), "/ax/z", L::new("Cota", "Elevation", "Cota"), "Start point elevation."), setting("bearing", Kind::Text, "/ax/bearing", L::new("Rumbo / azimut", "Bearing / azimuth", "Rumo / azimute"), "Start direction: azimuth in degrees or bearing (N45E, S30W, N10O)."), setting("grade", Kind::num(None, None, 0.5), "/ax/grade", L::new("Pendiente (+ sube)", "Grade (+ up)", "Inclinação (+ sobe)"), "Grade of the element (positive up)."), setting("grade_unit", Kind::Choice { options: &[opt("percent", L::new("%", "%", "%")), opt("degrees", L::new("°", "°", "°"))] }, "/ax/grade_unit", L::new("Unidad", "Unit", "Unidade"), "Unit of the grade."), setting("update_start", Kind::Bool, "/ax/update_start", L::new("Actualizar punto de inicio", "Update start point", "Atualizar ponto inicial"), "Continue the current axis: the element is appended and the start moves to its end. Off = each element is a new axis from the start point."), setting("curve_method", Kind::Choice { options: &[opt("length", L::new("Radio y longitud", "Radius and length", "Raio e comprimento")), opt("angle", L::new("Radio y ángulo", "Radius and angle", "Raio e ângulo"))] }, "/ax/curve_method", L::new("Definir por", "Define by", "Definir por"), "Curve given by radius and length, or radius and central angle."), setting("radius", Kind::num(Some(0.001), None, 1.0), "/ax/radius", L::new("Radio", "Radius", "Raio"), "Curve radius (m)."), setting("curve_length", Kind::num(Some(0.001), None, 1.0), "/ax/curve_length", L::new("Longitud de curva (horizontal)", "Curve length (horizontal)", "Comprimento da curva (horizontal)"), "Horizontal arc length (m), with curve_method = length."), setting("angle", Kind::num(Some(0.001), None, 5.0), "/ax/angle", L::new("Ángulo central (°)", "Central angle (°)", "Ângulo central (°)"), "Central angle (degrees), with curve_method = angle."), setting("turn", Kind::Choice { options: &[opt("right", L::new("Derecha (horario)", "Right (clockwise)", "Direita (horário)")), opt("left", L::new("Izquierda (antihorario)", "Left (counter-clockwise)", "Esquerda (anti-horário)"))] }, "/ax/turn", L::new("Sentido", "Direction", "Sentido"), "Turn direction."), setting("segment", Kind::num(Some(0.001), None, 0.5), "/ax/segment", L::new("Longitud de segmento", "Segment length", "Comprimento do segmento"), "Chord length used to draw the arc (m).")];

const AX_UNDO: &[Param] = &[];

const AX_NEW: &[Param] = &[];

const AX_PICKSTART: &[Param] = &[
    input(
        "point",
        Kind::Point,
        false,
        L::new("Punto inicial", "Start point", "Ponto inicial"),
        "Start point x,y,z. Without point and toward a pick is started.",
    ),
    input(
        "toward",
        Kind::Point,
        false,
        L::new("Dirección", "Direction point", "Ponto de direção"),
        "A second point giving the start azimuth.",
    ),
];

const AX_FROMPOLY: &[Param] = &[input(
    "polyline",
    Kind::Handle,
    false,
    L::new("Polilínea", "Polyline", "Polilinha"),
    "Polyline (or Open Mining axis) to continue. Without it a pick is started.",
)];

pub static OPS: &[OpDef] = &[
OpDef { name: "ax_line", command: "MIN_AX_LINE", aliases: &[],
            label: L::new("Agregar recta", "Add straight", "Adicionar reta"), group: Group::Axes, kind: OpKind::Action,
            summary: "Add a straight element (horizontal length + grade) to the axis being designed.", params: AX_LINE,
            positional: &[], result: "`axis` (id), `elements`, `length` (horizontal, m), `end` (`[x,y,z]`), `end_azimuth`." },
OpDef { name: "ax_curve", command: "MIN_AX_CURVE", aliases: &[],
            label: L::new("Agregar curva", "Add curve", "Adicionar curva"), group: Group::Axes, kind: OpKind::Action,
            summary: "Add a circular curve (radius + length or angle, right/left, with grade) to the axis being designed.", params: AX_CURVE,
            positional: &[], result: "`axis` (id), `elements`, `length` (horizontal, m), `end` (`[x,y,z]`), `end_azimuth`." },
OpDef { name: "ax_undo", command: "MIN_AX_UNDO", aliases: &[],
            label: L::new("Quitar último", "Remove last", "Remover último"), group: Group::Axes, kind: OpKind::Action,
            summary: "Remove the last element of the axis being designed.", params: AX_UNDO,
            positional: &[], result: "`axis`, `elements`, `length`, `end`, `end_azimuth` (all null when the axis became empty)." },
OpDef { name: "ax_new", command: "MIN_AX_NEW", aliases: &[],
            label: L::new("Nuevo eje", "New axis", "Novo eixo"), group: Group::Axes, kind: OpKind::Action,
            summary: "Close the current axis; the next element starts a new one.", params: AX_NEW,
            positional: &[], result: "empty object." },
OpDef { name: "ax_pickstart", command: "MIN_AX_PICKSTART", aliases: &[],
            label: L::new("Tomar del dibujo", "Pick from drawing", "Capturar do desenho"), group: Group::Axes, kind: OpKind::Action,
            summary: "Set the start point and direction of the axis designer from two points.", params: AX_PICKSTART,
            positional: &[], result: "`start` (`[x,y,z]`), `azimuth`." },
OpDef { name: "ax_frompoly", command: "MIN_AX_FROMPOLY", aliases: &[],
            label: L::new("Continuar polilínea", "Continue polyline", "Continuar polilinha"), group: Group::Axes, kind: OpKind::Action,
            summary: "Continue from a polyline: its end point and end azimuth become the start. An Open Mining axis is recovered from its XDATA and continued with its elements.", params: AX_FROMPOLY,
            positional: &[], result: "`start` (`[x,y,z]`), `azimuth`, `restored` (true when an Open Mining axis was recovered), `elements`." },
];

pub static PANELS: &[PanelSpec] = &[PanelSpec {
    id: "min_axes",
    title: L::new("Ejes de labores", "Working axes", "Eixos de escavações"),
    command: "MIN_AX_PANEL",
    items: &[
        Item::Section {
            id: "ax_start",
            title: L::new("Punto de inicio", "Start point", "Ponto inicial"),
            items: &[
                Item::Param("ax_line", "e"),
                Item::Param("ax_line", "n"),
                Item::Param("ax_line", "z"),
                Item::Param("ax_line", "bearing"),
            ],
            collapsed: false,
        },
        Item::Row(&[Item::Button("ax_pickstart"), Item::Button("ax_frompoly")]),
        Item::Section {
            id: "ax_grade_f",
            title: L::new("Pendiente", "Grade", "Inclinação"),
            items: &[Item::Param("ax_line", "grade"), Item::Param("ax_line", "grade_unit")],
            collapsed: false,
        },
        Item::Section {
            id: "ax_straight",
            title: L::new("Recta", "Straight", "Reta"),
            items: &[Item::Param("ax_line", "length"), Item::Button("ax_line")],
            collapsed: false,
        },
        Item::Section {
            id: "ax_curve",
            title: L::new("Curva", "Curve", "Curva"),
            items: &[
                Item::Param("ax_curve", "curve_method"),
                Item::Param("ax_curve", "radius"),
                Item::Param("ax_curve", "curve_length"),
                Item::Param("ax_curve", "angle"),
                Item::Param("ax_curve", "turn"),
                Item::Param("ax_curve", "segment"),
                Item::Button("ax_curve"),
            ],
            collapsed: false,
        },
        Item::Param("ax_line", "update_start"),
        Item::Row(&[Item::Button("ax_undo"), Item::Button("ax_new")]),
        Item::Status("ax_status"),
    ],
}];
