//! The plugin's own drawing abstraction. Commands build `Prim`s and talk to a
//! [`CadSink`]; [`crate::host_sink::HostSink`] maps it onto `HostApi`, and
//! [`mock::MockSink`] records calls for tests (no CAD needed).

use mining_core::Vec3;

use crate::xdata::Tag;

#[derive(Debug, Clone, PartialEq)]
pub enum Geom {
    Polyline3d(Vec<Vec3>),
    /// Closed 3D polyline (sections).
    Polygon3d(Vec<Vec3>),
    Line(Vec3, Vec3),
    Point(Vec3),
    Text {
        at: Vec3,
        height: f64,
        value: String,
    },
    Mesh {
        vertices: Vec<Vec3>,
        faces: Vec<Vec<usize>>,
    },
    /// Filled planar face (3 or 4 corners).
    Face(Vec<Vec3>),
}

/// One entity to create: geometry, layer, optional colour (ACI; `None` =
/// ByLayer) and the XDATA identity/attributes.
#[derive(Debug, Clone, PartialEq)]
pub struct Prim {
    pub geom: Geom,
    pub layer: String,
    pub color: Option<u8>,
    pub tag: Tag,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LayerDef {
    pub name: String,
    pub color: u8,
}

impl LayerDef {
    pub fn new(name: impl Into<String>, color: u8) -> Self {
        LayerDef {
            name: name.into(),
            color,
        }
    }
}

pub trait CadSink {
    fn ensure_layers(&mut self, layers: &[LayerDef]);
    /// Adds entities in batches; returns how many were created.
    fn add(&mut self, prims: Vec<Prim>) -> usize;
    /// Removes plugin entities whose tag matches; returns how many.
    fn remove_where(&mut self, pred: &dyn Fn(&Tag) -> bool) -> usize;
    fn begin(&mut self, label: &str);
    fn end(&mut self);
    fn info(&mut self, msg: &str);
    fn error(&mut self, msg: &str);
    /// Project settings stored in the drawing (JSON), if any.
    fn load_settings(&mut self) -> Option<String>;
    fn save_settings(&mut self, json: &str) -> Result<(), String>;
}

#[cfg(test)]
pub mod mock {
    use super::*;

    /// In-memory drawing for tests.
    #[derive(Default)]
    pub struct MockSink {
        pub entities: Vec<Prim>,
        pub layers: Vec<LayerDef>,
        pub infos: Vec<String>,
        pub errors: Vec<String>,
        pub settings: Option<String>,
        pub undo_depth: i32,
        pub undo_steps: usize,
    }

    impl CadSink for MockSink {
        fn ensure_layers(&mut self, layers: &[LayerDef]) {
            for l in layers {
                if !self.layers.iter().any(|x| x.name == l.name) {
                    self.layers.push(l.clone());
                }
            }
        }
        fn add(&mut self, prims: Vec<Prim>) -> usize {
            let n = prims.len();
            self.entities.extend(prims);
            n
        }
        fn remove_where(&mut self, pred: &dyn Fn(&Tag) -> bool) -> usize {
            let before = self.entities.len();
            self.entities.retain(|p| !pred(&p.tag));
            before - self.entities.len()
        }
        fn begin(&mut self, _label: &str) {
            self.undo_depth += 1;
            self.undo_steps += 1;
        }
        fn end(&mut self) {
            self.undo_depth -= 1;
        }
        fn info(&mut self, msg: &str) {
            self.infos.push(msg.to_string());
        }
        fn error(&mut self, msg: &str) {
            self.errors.push(msg.to_string());
        }
        fn load_settings(&mut self) -> Option<String> {
            self.settings.clone()
        }
        fn save_settings(&mut self, json: &str) -> Result<(), String> {
            self.settings = Some(json.to_string());
            Ok(())
        }
    }
}
