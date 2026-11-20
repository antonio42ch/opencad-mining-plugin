//! `CadSink` over the Open CAD Studio `HostApi`.

use ocs_plugin_api::host::acadrust::entities::{Face3D, Line, Mesh, MeshFace, Point, Polyline3D, Text};
use ocs_plugin_api::host::acadrust::objects::{XRecordEntry, XRecordValue};
use ocs_plugin_api::host::acadrust::types::{Color, Vector3};
use ocs_plugin_api::host::acadrust::xdata::{ExtendedDataRecord, XDataValue};
use ocs_plugin_api::host::acadrust::EntityType;
use ocs_plugin_api::host::{Handle, HostApi, LayerConfig};

use crate::cad::{CadSink, Geom, LayerDef, Prim};
use crate::xdata::{Tag, APP};

/// Entities per `add_entities` message (keeps each IPC frame well below the
/// host's 64 MiB cap even for dense meshes).
const BATCH: usize = 2_000;
/// Page size for `query_entities` (host maximum is 10 000).
const PAGE: u64 = 5_000;
const SETTINGS_DICT: &[&str] = &["OPENMINING"];
const SETTINGS_NAME: &str = "PROJECT";
/// DXF strings are limited in length; the JSON is split into chunks.
const CHUNK: usize = 1_800;

pub struct HostSink<'a> {
    pub host: &'a mut dyn HostApi,
    appid_registered: bool,
    /// Nested `begin`s join the outermost undo group: a command and the
    /// settings it saves are one undo step.
    depth: u32,
}

impl<'a> HostSink<'a> {
    pub fn new(host: &'a mut dyn HostApi) -> Self {
        HostSink {
            host,
            appid_registered: false,
            depth: 0,
        }
    }
}

fn v3(p: [f64; 3]) -> Vector3 {
    Vector3::new(p[0], p[1], p[2])
}

pub fn record(tag: &Tag) -> ExtendedDataRecord {
    ExtendedDataRecord {
        application_name: APP.to_string(),
        values: tag.encode().into_iter().map(XDataValue::String).collect(),
    }
}

pub fn tag_of(e: &EntityType) -> Option<Tag> {
    tag_of_app(e, APP)
}

/// Tag stored under XDATA application `app`.
pub fn tag_of_app(e: &EntityType, app: &str) -> Option<Tag> {
    let rec = e.common().extended_data.get_record(app)?;
    let strings: Vec<&str> = rec
        .values
        .iter()
        .filter_map(|v| {
            if let XDataValue::String(s) = v {
                Some(s.as_str())
            } else {
                None
            }
        })
        .collect();
    Tag::decode(strings)
}

pub fn record_for(app: &str, tag: &Tag) -> ExtendedDataRecord {
    ExtendedDataRecord {
        application_name: app.to_string(),
        values: tag.encode().into_iter().map(XDataValue::String).collect(),
    }
}

fn to_entity(p: Prim) -> EntityType {
    let mut e = match p.geom {
        Geom::Polyline3d(pts) => EntityType::Polyline3D(Polyline3D::from_points(pts.into_iter().map(v3).collect())),
        Geom::Polygon3d(pts) => {
            EntityType::Polyline3D(Polyline3D::from_points(pts.into_iter().map(v3).collect()).with_closed())
        }
        Geom::Line(a, b) => EntityType::Line(Line::from_points(v3(a), v3(b))),
        Geom::Point(a) => EntityType::Point(Point::from_coords(a[0], a[1], a[2])),
        Geom::Text { at, height, value } => EntityType::Text(Text::with_value(value, v3(at)).with_height(height)),
        Geom::Face(p) => {
            let c = |i: usize| v3(*p.get(i).or(p.last()).unwrap_or(&[0.0; 3]));
            EntityType::Face3D(Face3D::new(c(0), c(1), c(2), c(3)))
        }
        Geom::Mesh { vertices, faces } => {
            let mut m = Mesh::new();
            m.vertices = vertices.into_iter().map(v3).collect();
            m.faces = faces.into_iter().map(|f| MeshFace { vertices: f }).collect();
            EntityType::Mesh(m)
        }
    };
    let c = e.common_mut();
    c.layer = p.layer;
    if let Some(aci) = p.color {
        c.color = Color::from_index(aci as i16);
    }
    c.extended_data.add_record(record(&p.tag));
    e
}

impl CadSink for HostSink<'_> {
    fn ensure_layers(&mut self, layers: &[LayerDef]) {
        for l in layers {
            // Creating an existing layer is refused by the host and harmless.
            self.host.add_layer(LayerConfig {
                name: l.name.clone(),
                color: Some(Color::from_index(l.color as i16)),
                ..Default::default()
            });
        }
    }

    fn add(&mut self, prims: Vec<Prim>) -> usize {
        let mut created = 0;
        let mut it = prims.into_iter().peekable();
        while it.peek().is_some() {
            let chunk: Vec<Prim> = it.by_ref().take(BATCH).collect();
            let first_tag = chunk.first().map(|p| p.tag.clone());
            let handles = self.host.add_entities(chunk.into_iter().map(to_entity).collect());
            created += handles.iter().filter(|h| !h.is_null()).count();
            // Embedded XDATA travels with the entity; one `write_record`
            // registers the APPID so it survives DWG/DXF save.
            if !self.appid_registered {
                if let (Some(h), Some(tag)) = (handles.iter().find(|h| !h.is_null()), first_tag) {
                    self.appid_registered = self.host.write_record(*h, record(&tag));
                }
            }
        }
        created
    }

    fn remove_where(&mut self, pred: &dyn Fn(&Tag) -> bool) -> usize {
        let mut doomed: Vec<Handle> = Vec::new();
        let mut offset = 0u64;
        loop {
            let q = format!(r#"{{"xdata_app":"{APP}","offset":{offset},"limit":{PAGE}}}"#);
            let Ok(page) = self.host.query_entities(&q) else {
                break;
            };
            for e in &page.entities {
                if tag_of(e).is_some_and(|t| pred(&t)) {
                    doomed.push(e.common().handle);
                }
            }
            match page.next_offset {
                Some(n) if n > offset => offset = n,
                _ => break,
            }
        }
        doomed.into_iter().filter(|h| self.host.remove_entity(*h)).count()
    }

    fn begin(&mut self, label: &str) {
        if self.depth == 0 {
            self.host.begin_undo_group(label);
        }
        self.depth += 1;
    }
    fn end(&mut self) {
        self.depth = self.depth.saturating_sub(1);
        if self.depth == 0 {
            self.host.end_undo_group();
            self.host.set_dirty();
        }
    }
    fn info(&mut self, msg: &str) {
        self.host.push_info(msg);
    }
    fn error(&mut self, msg: &str) {
        self.host.push_error(msg);
    }

    fn load_settings(&mut self) -> Option<String> {
        let entries = self.host.read_xrecord(SETTINGS_DICT, SETTINGS_NAME).ok()?;
        let json: String = entries
            .iter()
            .filter_map(|e| {
                if let XRecordValue::String(s) = &e.value {
                    Some(s.as_str())
                } else {
                    None
                }
            })
            .collect();
        (!json.is_empty()).then_some(json)
    }

    fn save_settings(&mut self, json: &str) -> Result<(), String> {
        let chars: Vec<char> = json.chars().collect();
        let entries: Vec<XRecordEntry> = chars
            .chunks(CHUNK)
            .map(|c| XRecordEntry {
                code: 1,
                value: XRecordValue::String(c.iter().collect()),
            })
            .collect();
        match self.host.update_xrecord(SETTINGS_DICT, SETTINGS_NAME, entries.clone()) {
            Ok(_) => Ok(()),
            Err(_) => self
                .host
                .create_xrecord(SETTINGS_DICT, SETTINGS_NAME, entries)
                .map(|_| ()),
        }
    }
}
