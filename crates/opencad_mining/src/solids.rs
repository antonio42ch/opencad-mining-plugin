//! Underground workings: solids by axis or by four boundaries.

use mining_core::{Vec2, Vec3};
use solids_engine::axis::{axis_rings, AxisParams};
use solids_engine::boundary::{boundary_rings, Boundaries, BoundaryMode};
use solids_engine::error::SolidWarning;
use solids_engine::mesh::ClosedMesh;
use solids_engine::section::{alignment_offset, Profile};
use solids_engine::{Ring, SolidError};

use crate::cad::{CadSink, Geom, LayerDef, Prim};
use crate::config::WorkingsConfig;
use crate::i18n::{fmt, Strings};
use crate::xdata::{Class, Tag};

pub const LAYER_SOLID: &str = "MIN_UG_SOLIDO";
pub const LAYER_SECTION: &str = "MIN_UG_SECCION";

pub enum Method<'a> {
    Axis {
        axis: &'a [Vec3],
        custom_profile: Option<&'a [Vec2]>,
    },
    Boundaries {
        lines: [&'a [Vec3]; 4],
        vertical: bool,
    },
}

pub struct Built {
    pub rings: Vec<Ring>,
    pub mesh: ClosedMesh,
    pub warnings: Vec<SolidWarning>,
    pub method: &'static str,
}

pub fn build(cfg: &WorkingsConfig, m: Method) -> Result<Built, SolidError> {
    let (rings, warnings, method) = match m {
        Method::Axis { axis, custom_profile } => {
            let profile = match (cfg.profile(), custom_profile) {
                (Some(p), _) => p,
                (None, Some(pts)) => Profile::Custom(pts.to_vec()),
                (None, None) => return Err(SolidError::InvalidSection("custom profile polyline missing")),
            };
            let pts = profile.points()?;
            let (h, v) = cfg.align();
            let offset = alignment_offset(&pts, h, v);
            (
                axis_rings(&AxisParams {
                    axis,
                    profile: &profile,
                    interval: cfg.interval,
                    offset,
                })?,
                Vec::new(),
                "axis",
            )
        }
        Method::Boundaries { lines, vertical } => {
            let mode = if vertical {
                BoundaryMode::Vertical
            } else {
                BoundaryMode::Horizontal {
                    full_3d_tangent: cfg.tangent_3d,
                }
            };
            let r = boundary_rings(
                &Boundaries {
                    a: lines[0],
                    b: lines[1],
                    left: lines[2],
                    right: lines[3],
                    flips: [false; 4],
                },
                cfg.interval,
                mode,
            )?;
            (
                r.rings,
                r.warnings,
                if vertical { "boundaries_v" } else { "boundaries_h" },
            )
        }
    };
    let mesh = ClosedMesh::from_rings(&rings)?;
    Ok(Built {
        rings,
        mesh,
        warnings,
        method,
    })
}

pub fn scene(b: &Built, cfg: &WorkingsConfig) -> Vec<Prim> {
    let id = format!("{:x}", fingerprint(&b.mesh));
    let mut out = vec![Prim {
        geom: Geom::Mesh {
            vertices: b.mesh.vertices.clone(),
            faces: b.mesh.faces.clone(),
        },
        layer: LAYER_SOLID.into(),
        color: None,
        tag: Tag::new(Class::Solid)
            .with("id", &id)
            .with("method", b.method)
            .with("profile", cfg.profile)
            .with("interval", cfg.interval)
            .with("sections", b.rings.len())
            .with("volume", format!("{:.3}", b.mesh.volume())),
    }];
    if cfg.draw_sections {
        out.extend(b.rings.iter().enumerate().map(|(k, r)| Prim {
            geom: Geom::Polygon3d(r.clone()),
            layer: LAYER_SECTION.into(),
            color: None,
            tag: Tag::new(Class::Section).with("solid", &id).with("station", k),
        }));
    }
    out
}

/// Stable id linking a solid with its sections.
fn fingerprint(m: &ClosedMesh) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    for v in &m.vertices {
        for c in v {
            c.to_bits().hash(&mut h);
        }
    }
    h.finish()
}

/// Builds and draws the solid; `(volume m3, sections)` when it was created.
pub fn create(sink: &mut dyn CadSink, s: &Strings, cfg: &WorkingsConfig, m: Method) -> Option<(f64, usize)> {
    let built = match build(cfg, m) {
        Ok(b) => b,
        Err(e) => {
            sink.error(&e.to_string());
            return None;
        }
    };
    for w in &built.warnings {
        let SolidWarning::NoPlaneCrossing { station, boundary } = w;
        sink.info(&fmt(s.m_warn_no_crossing, &[boundary, station]));
    }
    sink.begin("MIN_UG_SOLID");
    sink.ensure_layers(&[LayerDef::new(LAYER_SOLID, 5), LayerDef::new(LAYER_SECTION, 4)]);
    sink.add(scene(&built, cfg));
    sink.end();
    sink.info(&fmt(
        s.m_solid_done,
        &[&built.rings.len(), &format!("{:.2}", built.mesh.volume())],
    ));
    Some((built.mesh.volume(), built.rings.len()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cad::mock::MockSink;
    use crate::i18n::{for_lang, Lang};

    fn create_ok(sink: &mut dyn CadSink, s: &Strings, cfg: &WorkingsConfig, m: Method) -> bool {
        create(sink, s, cfg, m).is_some()
    }

    #[test]
    fn axis_solid_with_sections() {
        let mut cfg = WorkingsConfig {
            draw_sections: true,
            interval: 5.0,
            ..Default::default()
        };
        cfg.width = 4.0;
        cfg.height = 3.0;
        let axis = [[0.0, 0.0, 0.0], [0.0, 20.0, 0.0]];
        let mut sink = MockSink::default();
        assert!(create_ok(
            &mut sink,
            for_lang(Lang::Es),
            &cfg,
            Method::Axis {
                axis: &axis,
                custom_profile: None
            }
        ));
        let solid = sink.entities.iter().find(|p| p.tag.class == Class::Solid).unwrap();
        assert_eq!(solid.tag.get("volume"), Some("240.000"));
        assert_eq!(
            sink.entities.iter().filter(|p| p.tag.class == Class::Section).count(),
            5
        );
        assert_eq!(sink.undo_depth, 0);
    }

    #[test]
    fn custom_profile_needs_polyline() {
        let cfg = WorkingsConfig {
            profile: 4,
            ..Default::default()
        };
        let axis = [[0.0; 3], [0.0, 10.0, 0.0]];
        let mut sink = MockSink::default();
        assert!(!create_ok(
            &mut sink,
            for_lang(Lang::Es),
            &cfg,
            Method::Axis {
                axis: &axis,
                custom_profile: None
            }
        ));
        assert_eq!(sink.errors.len(), 1);
        let tri = [[0.0, 0.0], [2.0, 0.0], [1.0, 2.0]];
        assert!(create_ok(
            &mut sink,
            for_lang(Lang::Es),
            &cfg,
            Method::Axis {
                axis: &axis,
                custom_profile: Some(&tri)
            }
        ));
    }
}
