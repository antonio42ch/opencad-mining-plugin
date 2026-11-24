//! Terrains: the TIN surfaces of Open CAD Studio's Civil tools (`AECC_TIN_SURFACE`).
//!
//! A terrain is read from the drawing as a triangulation and used as an ordinary
//! mesh by the sections (its cut is the terrain profile or its contours). The
//! plugin never changes a terrain.

use ocs_plugin_api::host::acadrust::classes::arch::ArchEntityData;
use ocs_plugin_api::host::acadrust::classes::civil::TinSurface;
use ocs_plugin_api::host::acadrust::entities::ExtendedEntityData;
use ocs_plugin_api::host::{EntityType, Handle, HostApi};
use serde_json::{json, Value};
use solids_engine::cut::MeshData;

use crate::i18n::{lang, Lang};

/// Class name of a Civil TIN surface in the drawing.
const CLASS: &str = "AECC_TIN_SURFACE";
/// The entry that means "no terrain" in the lists.
pub const NONE: &str = "—";

pub struct TrStrings {
    pub unnamed: &'static str,
    pub m_found: &'static str,
    pub m_none: &'static str,
    pub m_unknown: &'static str,
    pub m_empty: &'static str,
    pub m_profile: &'static str,
    pub m_missed: &'static str,
    pub label: &'static str,
}

static ES: TrStrings = TrStrings {
    unnamed: "Terreno",
    m_found: "Terrenos en el dibujo: {0}.",
    m_none: "No hay terrenos en el dibujo (superficies TIN de Civil: CREATESURFACE).",
    m_unknown: "Terreno '{0}': no está en el dibujo. Terrenos: {1}.",
    m_empty: "El terreno '{0}' no tiene triángulos visibles.",
    m_profile: "Terreno '{0}': {1} planos cortan el terreno, {2} polilíneas, cota {3} a {4}.",
    m_missed: "El terreno no cruza los planos: {0}.",
    label: "Terreno: {0} – {1}",
};

static EN: TrStrings = TrStrings {
    unnamed: "Terrain",
    m_found: "Terrains in the drawing: {0}.",
    m_none: "There are no terrains in the drawing (Civil TIN surfaces: CREATESURFACE).",
    m_unknown: "Terrain '{0}' is not in the drawing. Terrains: {1}.",
    m_empty: "Terrain '{0}' has no visible triangles.",
    m_profile: "Terrain '{0}': {1} planes cut the terrain, {2} polylines, elevation {3} to {4}.",
    m_missed: "The terrain does not cross the planes: {0}.",
    label: "Terrain: {0} – {1}",
};

static PT: TrStrings = TrStrings {
    unnamed: "Terreno",
    m_found: "Terrenos no desenho: {0}.",
    m_none: "Não há terrenos no desenho (superfícies TIN do Civil: CREATESURFACE).",
    m_unknown: "Terreno '{0}': não está no desenho. Terrenos: {1}.",
    m_empty: "O terreno '{0}' não tem triângulos visíveis.",
    m_profile: "Terreno '{0}': {1} planos cortam o terreno, {2} polilinhas, cota {3} a {4}.",
    m_missed: "O terreno não cruza os planos: {0}.",
    label: "Terreno: {0} – {1}",
};

pub fn t() -> &'static TrStrings {
    match lang() {
        Lang::Es => &ES,
        Lang::En => &EN,
        Lang::Pt => &PT,
    }
}

/// A TIN surface of the drawing, ready to cut.
pub struct Terrain {
    pub handle: Handle,
    pub name: String,
    /// Triangles drawn (the hidden and deleted ones are left out).
    pub triangles: usize,
    pub vertices: usize,
    pub min: [f64; 3],
    pub max: [f64; 3],
    pub mesh: MeshData,
}

fn arch(e: &EntityType) -> Option<&ArchEntityData> {
    match e {
        EntityType::Extended(x) => match &x.data {
            ExtendedEntityData::Arch(a) if a.dxf_name == CLASS => Some(a),
            _ => None,
        },
        _ => None,
    }
}

/// The surface's name stored in its record, when one can be told apart.
fn stored_name(a: &ArchEntityData) -> Option<String> {
    a.decode().strings.into_iter().find(|s| {
        !s.trim().is_empty() && !s.contains(".dwg") && !s.starts_with("AECC") && !s.starts_with("_") && s.len() <= 64
    })
}

/// The visible triangles of a surface as a mesh with shared vertices.
fn mesh_of(tin: &TinSurface) -> (MeshData, usize, usize) {
    let mut index = vec![usize::MAX; tin.vertices.len()];
    let mut vertices: Vec<[f64; 3]> = Vec::new();
    let mut faces: Vec<Vec<usize>> = Vec::new();
    for tri in tin.triangles.iter().filter(|t| t.is_visible()) {
        let mut face = Vec::with_capacity(3);
        for &v in &tri.vertices {
            let v = v as usize;
            let Some(src) = tin.vertices.get(v) else { break };
            if index[v] == usize::MAX {
                index[v] = vertices.len();
                vertices.push([src.position.x, src.position.y, src.position.z]);
            }
            face.push(index[v]);
        }
        if face.len() == 3 {
            faces.push(face);
        }
    }
    let used = vertices.len();
    let n = faces.len();
    (MeshData { vertices, faces }, used, n)
}

/// Every terrain of the drawing (named `Terreno <handle>` when the record keeps no name).
pub fn scan(host: &mut dyn HostApi) -> Vec<Terrain> {
    let mut out: Vec<Terrain> = Vec::new();
    let mut offset = 0u64;
    loop {
        let q = json!({"type": CLASS, "offset": offset, "limit": 200}).to_string();
        let Ok(page) = host.query_entities(&q) else { break };
        for e in &page.entities {
            let Some(a) = arch(e) else { continue };
            let Some(tin) = a.tin_surface().or_else(|| a.tin_snapshot()) else {
                continue;
            };
            let (mesh, vertices, triangles) = mesh_of(&tin);
            let Some((min, max)) = mesh.bounds() else { continue };
            let handle = e.common().handle;
            let name = stored_name(a).unwrap_or_else(|| format!("{} {:X}", t().unnamed, handle.value()));
            out.push(Terrain {
                handle,
                name,
                triangles,
                vertices,
                min,
                max,
                mesh,
            });
        }
        match page.next_offset {
            Some(n) if n > offset => offset = n,
            _ => break,
        }
    }
    // Two surfaces with the same name stay apart.
    for i in 1..out.len() {
        if out[..i].iter().any(|o| o.name.eq_ignore_ascii_case(&out[i].name)) {
            out[i].name = format!("{} ({:X})", out[i].name, out[i].handle.value());
        }
    }
    out
}

/// A terrain by name (without case) or by hexadecimal handle.
pub fn find<'a>(list: &'a [Terrain], key: &str) -> Option<&'a Terrain> {
    let k = key.trim();
    list.iter().find(|t| t.name.eq_ignore_ascii_case(k)).or_else(|| {
        u64::from_str_radix(k, 16)
            .ok()
            .and_then(|h| list.iter().find(|t| t.handle.value() == h))
    })
}

/// Whether a setting value means "no terrain".
pub fn is_none(name: &str) -> bool {
    let n = name.trim();
    n.is_empty() || n == NONE
}

/// Scans the drawing and keeps the names for the panels' lists.
pub fn refresh(host: &mut dyn HostApi, tab: u64) -> Vec<Terrain> {
    let list = scan(host);
    let names: Vec<String> = std::iter::once(NONE.to_string())
        .chain(list.iter().map(|t| t.name.clone()))
        .collect();
    crate::state::with_tab(tab, |s| s.terrains = names);
    list
}

pub fn info(t: &Terrain) -> Value {
    json!({
        "handle": format!("{:X}", t.handle.value()),
        "name": t.name,
        "vertices": t.vertices,
        "triangles": t.triangles,
        "min": t.min,
        "max": t.max,
        "min_elevation": t.min[2],
        "max_elevation": t.max[2],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ocs_plugin_api::host::acadrust::classes::civil::{TinTriangle, TinVertex};
    use ocs_plugin_api::host::acadrust::types::Vector3;

    fn tin() -> TinSurface {
        let v = |x: f64, y: f64, z: f64, flags: i32| TinVertex {
            position: Vector3::new(x, y, z),
            flags,
            triangle: 0,
        };
        let tri = |a: u32, b: u32, c: u32, flags: i32| TinTriangle {
            vertices: [a, b, c],
            neighbors: [-1; 3],
            flags,
        };
        TinSurface {
            vertices: vec![
                v(0.0, 0.0, 10.0, 0x80),
                v(10.0, 0.0, 12.0, 0x80),
                v(10.0, 10.0, 14.0, 0x80),
                v(0.0, 10.0, 11.0, 0x80),
                v(-99.0, -99.0, 0.0, 0x40),
            ],
            triangles: vec![tri(0, 1, 2, 0), tri(0, 2, 3, 0), tri(0, 1, 4, 1), tri(1, 2, 3, 4)],
            corners: [4, 4, 4, 4],
        }
    }

    #[test]
    fn only_visible_triangles_make_the_mesh_and_vertices_are_shared() {
        let (mesh, vertices, triangles) = mesh_of(&tin());
        // The hidden (flag 1) and deleted (flag 4) triangles and the corner vertex are left out.
        assert_eq!((vertices, triangles), (4, 2));
        assert_eq!(mesh.faces.len(), 2);
        let (lo, hi) = mesh.bounds().unwrap();
        assert_eq!((lo[2], hi[2]), (10.0, 14.0));
    }

    #[test]
    fn terrains_are_found_by_name_without_case_or_by_handle() {
        let mk = |name: &str, h: u64| Terrain {
            handle: Handle::new(h),
            name: name.into(),
            triangles: 1,
            vertices: 3,
            min: [0.0; 3],
            max: [1.0; 3],
            mesh: MeshData {
                vertices: vec![],
                faces: vec![],
            },
        };
        let list = vec![mk("Topografía", 0x1F), mk("Diseño", 0x20)];
        assert_eq!(find(&list, "topografía").map(|t| t.handle.value()), Some(0x1F));
        assert_eq!(find(&list, "20").map(|t| t.name.as_str()), Some("Diseño"));
        assert!(find(&list, "nada").is_none());
        assert!(is_none("") && is_none(NONE) && !is_none("Topografía"));
    }
}
