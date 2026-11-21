//! Narrow-vein operations (`vn_*`): stage B1, hanging-wall and footwall
//! surfaces built from mapped contacts and vein intercepts.

use geostat_engine::surface::GridSurface;
use ocs_plugin_api::host::acadrust::EntityType;
use ocs_plugin_api::host::{Handle, HostApi};
use openmining_ops::parse::Args;
use openmining_ops::result::{ErrorCode, OpError, OpResult};
use serde_json::{json, Value};
use vein_engine::contacts::{densify, intercepts, Contact, Side};
use vein_engine::frame::{fit, VeinFrame};
use vein_engine::surfaces::{build, intercept_contacts, mesh, Interp, Params, SurfError, Surfaces};

use crate::cad::{CadSink, Geom, LayerDef, Prim};
use crate::config::ProjectConfig;
use crate::host_sink::HostSink;
use crate::i18n::{fmt, lang, Lang};
use crate::opsrun::Outcome;
use crate::session;
use crate::state;
use crate::xdata::{Class, Tag};

pub const PANEL: &str = "min_vein";
const STATUS: &str = "vn_status";
pub const LAYER_HW: &str = "MIN_VN_TECHO";
pub const LAYER_FW: &str = "MIN_VN_PISO";

pub struct VnStrings {
    pub grp: &'static str,
    pub tool_panel: &'static str,
    pub tool_surfaces: &'static str,
    pub cls_surface: &'static str,
    pub m_few: &'static str,
    pub m_nodes: &'static str,
    pub m_plane: &'static str,
    pub m_thickness: &'static str,
    pub m_rms: &'static str,
    pub m_crossings: &'static str,
    pub m_one_side: &'static str,
    pub m_worst: &'static str,
    pub m_saved: &'static str,
    pub m_no_plane: &'static str,
    pub sides: [&'static str; 2],
}

pub static ES: VnStrings = VnStrings {
    grp: "Vetas",
    tool_panel: "Veta",
    tool_surfaces: "Superficies",
    cls_surface: "Superficie de veta",
    m_few: "Veta {0}: {1} puntos de contacto de {2} (se necesitan al menos 3). Revise la capa, la selección o las intersecciones ({3} = {4}).",
    m_nodes: "Malla de {0} × {1} nodos ({2} m); se usó el radio para limitar la extrapolación.",
    m_plane: "Veta {0}: plano {1}/{2} (buzamiento/dirección); techo {3} puntos, piso {4} puntos ({5} de capas, {6} seleccionados, {7} de intersecciones).",
    m_thickness: "Potencia entre superficies: mín {0} m, media {1} m, máx {2} m en {3} nodos.",
    m_rms: "Ajuste a los contactos (RMS): techo {0} m, piso {1} m.",
    m_crossings: "{0} nodos con el piso por encima del techo; revise los contactos cercanos a {1}.",
    m_one_side: "{0} nodos con una sola superficie: amplíe el radio o agregue contactos en esa zona.",
    m_worst: "Contacto más alejado de su superficie: {0} ({1}), {2} m.",
    m_saved: "Superficies guardadas en {0}.",
    m_no_plane: "No se pudo ajustar un plano: los contactos están alineados. Escriba el buzamiento y la dirección.",
    sides: ["techo", "piso"],
};

pub static EN: VnStrings = VnStrings {
    grp: "Veins",
    tool_panel: "Vein",
    tool_surfaces: "Surfaces",
    cls_surface: "Vein surface",
    m_few: "Vein {0}: {1} contact point(s) on the {2} (at least 3 are needed). Check the layer, the selection or the intercepts ({3} = {4}).",
    m_nodes: "Grid of {0} × {1} nodes ({2} m); the radius limits the extrapolation.",
    m_plane: "Vein {0}: plane {1}/{2} (dip/dip direction); hanging wall {3} points, footwall {4} points ({5} from layers, {6} picked, {7} from intercepts).",
    m_thickness: "Thickness between the surfaces: min {0} m, mean {1} m, max {2} m on {3} nodes.",
    m_rms: "Fit to the contacts (RMS): hanging wall {0} m, footwall {1} m.",
    m_crossings: "{0} nodes with the footwall above the hanging wall; check the contacts near {1}.",
    m_one_side: "{0} nodes with one surface only: widen the radius or add contacts there.",
    m_worst: "Contact farthest from its surface: {0} ({1}), {2} m.",
    m_saved: "Surfaces saved to {0}.",
    m_no_plane: "No plane could be fitted: the contacts are aligned. Type the dip and dip direction.",
    sides: ["hanging wall", "footwall"],
};

pub static PT: VnStrings = VnStrings {
    grp: "Veios",
    tool_panel: "Veio",
    tool_surfaces: "Superfícies",
    cls_surface: "Superfície de veio",
    m_few: "Veio {0}: {1} ponto(s) de contato na {2} (são necessários ao menos 3). Verifique a camada, a seleção ou as interseções ({3} = {4}).",
    m_nodes: "Malha de {0} × {1} nós ({2} m); o raio limita a extrapolação.",
    m_plane: "Veio {0}: plano {1}/{2} (mergulho/direção); capa {3} pontos, lapa {4} pontos ({5} de camadas, {6} selecionados, {7} de interseções).",
    m_thickness: "Espessura entre as superfícies: mín {0} m, média {1} m, máx {2} m em {3} nós.",
    m_rms: "Ajuste aos contatos (RMS): capa {0} m, lapa {1} m.",
    m_crossings: "{0} nós com a lapa acima da capa; verifique os contatos perto de {1}.",
    m_one_side: "{0} nós com uma só superfície: aumente o raio ou adicione contatos ali.",
    m_worst: "Contato mais distante da sua superfície: {0} ({1}), {2} m.",
    m_saved: "Superfícies salvas em {0}.",
    m_no_plane: "Não foi possível ajustar um plano: os contatos estão alinhados. Digite o mergulho e a direção.",
    sides: ["capa", "lapa"],
};

pub fn v() -> &'static VnStrings {
    match lang() {
        Lang::Es => &ES,
        Lang::En => &EN,
        Lang::Pt => &PT,
    }
}

/// Surfaces of the tab's active vein.
#[derive(Debug, Clone, Default)]
pub struct VeinState {
    pub name: String,
    pub surfaces: Option<Surfaces>,
}

fn side_text(s: Side) -> &'static str {
    v().sides[usize::from(s == Side::Foot)]
}

fn handles(args: &Args, key: &str) -> Vec<Handle> {
    crate::gtcommands::handles_arg(args, key).unwrap_or_default()
}

/// Polylines on a layer (wildcards), as world points per polyline.
fn layer_lines(host: &mut dyn HostApi, layer: &str) -> Vec<(String, Vec<mining_core::Vec3>)> {
    let mut out = Vec::new();
    if layer.trim().is_empty() {
        return out;
    }
    let mut offset = 0u64;
    loop {
        let q = json!({"layer": layer.trim(), "offset": offset, "limit": 5000}).to_string();
        let Ok(page) = host.query_entities(&q) else { break };
        for e in &page.entities {
            if let Some(p) = crate::polyline_points(e) {
                out.push((format!("{}:{:X}", layer.trim(), e.common().handle.value()), p));
            }
        }
        match page.next_offset {
            Some(n) if n > offset => offset = n,
            _ => break,
        }
    }
    out
}

fn picked_lines(host: &dyn HostApi, hs: &[Handle]) -> Vec<(String, Vec<mining_core::Vec3>)> {
    let ents = host.entities_batch(hs).unwrap_or_default();
    hs.iter()
        .zip(ents)
        .filter_map(|(h, e)| {
            let e: EntityType = e?;
            Some((format!("{:X}", h.value()), crate::polyline_points(&e)?))
        })
        .collect()
}

fn grid_json(g: &GridSurface) -> Value {
    json!({"x0": g.x0, "y0": g.y0, "dx": g.dx, "dy": g.dy, "nx": g.nx, "ny": g.ny, "z": g.z.iter().map(|z| if z.is_finite() { json!(z) } else { Value::Null }).collect::<Vec<_>>()})
}

fn grid_from(v: &Value) -> Option<GridSurface> {
    let f = |k: &str| v.get(k).and_then(Value::as_f64);
    let u = |k: &str| v.get(k).and_then(Value::as_u64).map(|x| x as usize);
    let z: Vec<f64> = v
        .get("z")?
        .as_array()?
        .iter()
        .map(|x| x.as_f64().unwrap_or(f64::NAN))
        .collect();
    let (nx, ny) = (u("nx")?, u("ny")?);
    (z.len() == nx * ny).then_some(GridSurface {
        x0: f("x0")?,
        y0: f("y0")?,
        dx: f("dx")?,
        dy: f("dy")?,
        nx,
        ny,
        z,
    })
}

/// Surfaces as JSON (saved next to the drawing for the prism model).
pub fn surfaces_json(name: &str, s: &Surfaces) -> Value {
    json!({"format": "openmining-vein-surfaces", "version": 1, "vein": name,
           "frame": {"origin": s.frame.origin, "e1": s.frame.e1, "e2": s.frame.e2, "n": s.frame.n},
           "hanging_wall": grid_json(&s.hw), "footwall": grid_json(&s.fw)})
}

pub fn surfaces_from(v: &Value) -> Option<Surfaces> {
    let fr = v.get("frame")?;
    let vec = |k: &str| -> Option<mining_core::Vec3> {
        let a = fr.get(k)?.as_array()?;
        Some([a.first()?.as_f64()?, a.get(1)?.as_f64()?, a.get(2)?.as_f64()?])
    };
    let frame = VeinFrame {
        origin: vec("origin")?,
        e1: vec("e1")?,
        e2: vec("e2")?,
        n: vec("n")?,
    };
    Some(Surfaces {
        frame,
        hw: grid_from(v.get("hanging_wall")?)?,
        fw: grid_from(v.get("footwall")?)?,
    })
}

pub fn surface_file(host: &dyn HostApi, name: &str) -> Option<std::path::PathBuf> {
    let safe: String = name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' { c } else { '_' })
        .collect();
    crate::resources::model_file(
        host.document_path(host.tab_id()).as_deref(),
        "",
        &format!("_veta_{safe}.json"),
    )
}

fn surfaces_op(host: &mut dyn HostApi, args: &Args, tab: u64, cfg: &ProjectConfig) -> Outcome {
    let s = v();
    let x = &cfg.vn;
    let origin = crate::drillholes::origin(cfg);
    // Mapped contacts are drawing entities: back to world coordinates.
    let world = |p: mining_core::Vec3| origin.to_world(p);
    let mut contacts: Vec<Contact> = Vec::new();
    let (mut from_layers, mut from_handles) = (0usize, 0usize);
    for (side, layer, key) in [(Side::Hanging, &x.hw_layer, "hw"), (Side::Foot, &x.fw_layer, "fw")] {
        for (src, pts) in layer_lines(host, layer) {
            for p in densify(&pts, x.densify) {
                contacts.push(Contact {
                    p: world(p),
                    side,
                    source: src.clone(),
                });
                from_layers += 1;
            }
        }
        for (src, pts) in picked_lines(host, &handles(args, key)) {
            for p in densify(&pts, x.densify) {
                contacts.push(Contact {
                    p: world(p),
                    side,
                    source: src.clone(),
                });
                from_handles += 1;
            }
        }
    }
    let ints = if x.use_intercepts {
        let data = state::with_tab(tab, |t| t.data.clone());
        data.map(|d| intercepts(&drillhole_engine::pipeline::build_holes(&d), &x.vein_attr, &x.values()))
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    // Reference plane: typed, or fitted to every contact (walls and intercepts).
    let frame = if x.orient_mode == 1 {
        let pts: Vec<_> = contacts
            .iter()
            .map(|c| c.p)
            .chain(ints.iter().flat_map(|i| [i.a, i.b]))
            .collect();
        let n = pts.len().max(1) as f64;
        let c: mining_core::Vec3 = std::array::from_fn(|k| pts.iter().map(|p| p[k]).sum::<f64>() / n);
        VeinFrame::from_orientation(c, x.dip, x.dipdir)
    } else {
        let pts: Vec<_> = contacts
            .iter()
            .map(|c| c.p)
            .chain(ints.iter().flat_map(|i| [i.a, i.b]))
            .collect();
        fit(&pts)
    }
    .ok_or_else(|| OpError::new(ErrorCode::NoData, s.m_no_plane))?;
    let from_ints = ints.len() * 2;
    contacts.extend(intercept_contacts(&frame, &ints));
    let interp = if x.method == 1 {
        Interp::Idw { power: x.power }
    } else {
        Interp::Kriging { range: x.range }
    };
    let p = Params {
        step: x.step,
        margin: x.margin,
        radius: x.radius,
        min_samples: x.min_samples,
        max_samples: x.max_samples,
        interp,
        min_thickness: x.min_thickness,
    };
    let (surf, d) = build(&contacts, frame, &p).map_err(|e| match e {
        SurfError::TooFewContacts { side, found } => OpError::new(
            ErrorCode::NoData,
            fmt(
                s.m_few,
                &[&x.name, &found, &side_text(side), &x.vein_attr, &x.vein_values],
            ),
        ),
        other => OpError::new(ErrorCode::OutOfRange, other.to_string()).at("step"),
    })?;
    let (dip, dd) = surf.frame.orientation();
    let mut lines = vec![
        fmt(
            s.m_plane,
            &[
                &x.name,
                &format!("{dip:.0}"),
                &format!("{dd:03.0}"),
                &d.hw_points,
                &d.fw_points,
                &from_layers,
                &from_handles,
                &from_ints,
            ],
        ),
        fmt(s.m_nodes, &[&surf.hw.nx, &surf.hw.ny, &x.step]),
    ];
    if d.both > 0 {
        lines.push(fmt(
            s.m_thickness,
            &[
                &format!("{:.2}", d.thickness_min),
                &format!("{:.2}", d.thickness_mean),
                &format!("{:.2}", d.thickness_max),
                &d.both,
            ],
        ));
    }
    lines.push(fmt(
        s.m_rms,
        &[&format!("{:.3}", d.rms_hw), &format!("{:.3}", d.rms_fw)],
    ));
    let mut warnings = Vec::new();
    if d.crossings > 0 {
        let near = d.worst.first().map_or("-".to_string(), |w| w.source.clone());
        let w = fmt(s.m_crossings, &[&d.crossings, &near]);
        warnings.push(w.clone());
        lines.push(w);
    }
    if d.one_side > 0 {
        let w = fmt(s.m_one_side, &[&d.one_side]);
        warnings.push(w.clone());
        lines.push(w);
    }
    if let Some(w) = d.worst.first() {
        lines.push(fmt(
            s.m_worst,
            &[&w.source, &side_text(w.side), &format!("{:.3}", w.residual)],
        ));
    }
    // Draw both walls, replacing the earlier drawing of this vein.
    let name = x.name.trim().to_string();
    let prims: Vec<Prim> = [(Side::Hanging, LAYER_HW, 30u8), (Side::Foot, LAYER_FW, 4u8)]
        .iter()
        .filter_map(|(side, layer, color)| {
            let (verts, faces) = mesh(&surf, *side);
            (!faces.is_empty()).then(|| Prim {
                geom: Geom::Mesh {
                    vertices: verts.into_iter().map(|p| origin.to_local(p)).collect(),
                    faces,
                },
                layer: layer.to_string(),
                color: Some(*color),
                tag: Tag::new(Class::VnSurface).with("vein", &name).with("side", side.key()),
            })
        })
        .collect();
    let file = surface_file(host, &name).and_then(|f| {
        std::fs::write(&f, surfaces_json(&name, &surf).to_string())
            .ok()
            .map(|_| f.display().to_string())
    });
    if let Some(f) = &file {
        lines.push(fmt(s.m_saved, &[f]));
    }
    lines.iter().for_each(|l| host.push_info(l));
    let mut sink = HostSink::new(host);
    sink.begin("MIN_VN_SURFACES");
    sink.ensure_layers(&[LayerDef::new(LAYER_HW, 30), LayerDef::new(LAYER_FW, 4)]);
    sink.remove_where(&|t: &Tag| t.class == Class::VnSurface && t.get("vein") == Some(name.as_str()));
    sink.add(prims);
    session::commit(&mut sink, crate::i18n::s(), tab, cfg);
    sink.end();
    let created = std::mem::take(&mut sink.created);
    crate::opsbind::push_one(host, cfg, PANEL, Some((STATUS, lines.join("\n"))));
    let worst: Vec<Value> = d
        .worst
        .iter()
        .map(|w| json!({"source": w.source, "side": w.side.key(), "residual": w.residual, "point": w.point}))
        .collect();
    let fin = |x: f64| if x.is_finite() { json!(x) } else { Value::Null };
    let data = json!({
        "vein": name,
        "plane": {"dip": dip, "dipdir": dd, "origin": surf.frame.origin},
        "contacts": {"hanging_wall": d.hw_points, "footwall": d.fw_points, "from_layers": from_layers, "from_handles": from_handles, "from_intercepts": from_ints},
        "grid": {"nx": surf.hw.nx, "ny": surf.hw.ny, "step": x.step, "nodes": d.nodes},
        "diagnostics": {"both": d.both, "one_side": d.one_side, "crossings": d.crossings, "thin": d.thin,
            "thickness_min": fin(d.thickness_min), "thickness_mean": fin(d.thickness_mean), "thickness_max": fin(d.thickness_max),
            "rms_hw": fin(d.rms_hw), "rms_fw": fin(d.rms_fw), "unchecked": d.unchecked},
        "worst": worst,
        "file": file,
    });
    state::with_tab(tab, |t| {
        t.vein = VeinState {
            name,
            surfaces: Some(surf),
        }
    });
    Ok(OpResult {
        created,
        warnings,
        ..OpResult::data(data)
    })
}

pub fn run(host: &mut dyn HostApi, name: &str, args: &Args, tab: u64, cfg: &mut ProjectConfig) -> Outcome {
    match name {
        "vn_surfaces" => surfaces_op(host, args, tab, cfg),
        "vn_variogram" | "vn_estimate" | "vn_long" | "vn_edit" => crate::veinmodel::run(host, name, args, tab, cfg),
        other => Err(OpError::new(
            ErrorCode::UnknownCommand,
            format!("unknown vein operation {other}"),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn surfaces_survive_a_json_round_trip() {
        let f = VeinFrame::from_orientation([1.0, 2.0, 3.0], 70.0, 90.0).unwrap();
        let g = GridSurface {
            x0: -1.0,
            y0: -2.0,
            dx: 1.0,
            dy: 1.0,
            nx: 2,
            ny: 2,
            z: vec![0.5, f64::NAN, 0.7, 0.8],
        };
        let s = Surfaces {
            frame: f,
            hw: g.clone(),
            fw: GridSurface {
                z: vec![-0.5, -0.4, f64::NAN, -0.2],
                ..g
            },
        };
        let back = surfaces_from(&surfaces_json("V1", &s)).unwrap();
        assert_eq!(back.frame, s.frame);
        assert_eq!((back.hw.nx, back.fw.ny), (2, 2));
        assert!(back.hw.z[1].is_nan() && back.fw.z[2].is_nan() && back.hw.z[3] == 0.8);
        assert!(surfaces_from(&json!({"frame": {}})).is_none());
    }
}
