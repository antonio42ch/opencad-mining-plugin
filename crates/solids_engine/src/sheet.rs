//! Lays out 2D sections as sheets on a grid: one frame per section with a
//! coordinate grid, axis labels, title and area. Pure geometry; the plugin
//! turns the items into CAD entities.

use mining_core::Vec2;

use crate::cut::SectionLoop;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SheetConfig {
    /// Lower-left corner of the first sheet (drawing units).
    pub origin: Vec2,
    pub columns: usize,
    /// Space between sheets.
    pub gap: f64,
    /// Grid spacing (also rounds the frame to whole grid steps).
    pub grid: f64,
    pub text_height: f64,
}

/// What an item represents (for layers / XDATA).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Frame,
    Grid,
    GridLabel,
    Title,
    Area,
    Section,
    Marker,
    /// Terrain profile or contour (open or closed polyline).
    Terrain,
    /// Elevation range of the terrain on a sheet.
    TerrainLabel,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SheetGeom {
    Polyline { points: Vec<Vec2>, closed: bool },
    Text { at: Vec2, height: f64, value: String },
}

#[derive(Debug, Clone, PartialEq)]
pub struct SheetItem {
    pub sheet: usize,
    pub role: Role,
    pub geom: SheetGeom,
}

/// One section to lay out.
pub struct SheetInput<'a> {
    pub title: &'a str,
    pub loops: &'a [SectionLoop],
    pub marker: Option<Vec2>,
    /// The terrain cut by the same plane (already limited to the window).
    pub terrain: &'a [SectionLoop],
}

fn fmt_coord(v: f64) -> String {
    if v.fract().abs() < 1e-9 {
        format!("{v:.0}")
    } else {
        format!("{v:.2}")
    }
}

/// Builds every sheet. `area_label` formats the total closed area.
/// `terrain_label` formats the lowest and highest elevation of a sheet's terrain.
pub fn layout(
    inputs: &[SheetInput],
    cfg: &SheetConfig,
    area_label: &dyn Fn(f64) -> String,
    terrain_label: &dyn Fn(f64, f64) -> String,
) -> Vec<SheetItem> {
    let g = if cfg.grid > 0.0 { cfg.grid } else { 1.0 };
    let h = if cfg.text_height > 0.0 {
        cfg.text_height
    } else {
        g / 5.0
    };
    let cols = cfg.columns.max(1);
    // Grid range of each section, snapped outwards to whole steps (+1 margin).
    let ranges: Vec<Option<(Vec2, Vec2)>> = inputs
        .iter()
        .map(|s| {
            let pts = s
                .loops
                .iter()
                .chain(s.terrain)
                .flat_map(|l| l.points_2d.iter().copied())
                .chain(s.marker);
            let (mut lo, mut hi) = ([f64::MAX; 2], [f64::MIN; 2]);
            let mut any = false;
            for p in pts {
                any = true;
                lo = [lo[0].min(p[0]), lo[1].min(p[1])];
                hi = [hi[0].max(p[0]), hi[1].max(p[1])];
            }
            any.then(|| {
                let snap_lo = |v: f64| ((v / g).floor() - 1.0) * g;
                let snap_hi = |v: f64| ((v / g).ceil() + 1.0) * g;
                ([snap_lo(lo[0]), snap_lo(lo[1])], [snap_hi(hi[0]), snap_hi(hi[1])])
            })
        })
        .collect();
    // Uniform cells: the largest frame, plus room for labels and titles.
    let (cw, ch) = ranges.iter().flatten().fold((0.0f64, 0.0f64), |(w, hh), (lo, hi)| {
        (w.max(hi[0] - lo[0]), hh.max(hi[1] - lo[1]))
    });
    let label_w = 6.0 * h;
    let cell_w = cw + label_w + cfg.gap;
    let cell_h = ch + 5.0 * h + cfg.gap;

    let mut out = Vec::new();
    let mut slot = 0usize;
    for (i, (s, r)) in inputs.iter().zip(&ranges).enumerate() {
        let Some((lo, hi)) = *r else { continue };
        let (col, row) = (slot % cols, slot / cols);
        slot += 1;
        // Frame lower-left in drawing coordinates.
        let x0 = cfg.origin[0] + label_w + col as f64 * cell_w;
        let y0 = cfg.origin[1] + 2.0 * h - row as f64 * cell_h;
        let map = |p: Vec2| [x0 + (p[0] - lo[0]), y0 + (p[1] - lo[1])];
        let (w, hh) = (hi[0] - lo[0], hi[1] - lo[1]);
        let push = |out: &mut Vec<SheetItem>, role, geom| out.push(SheetItem { sheet: i, role, geom });

        push(
            &mut out,
            Role::Frame,
            SheetGeom::Polyline {
                points: vec![[x0, y0], [x0 + w, y0], [x0 + w, y0 + hh], [x0, y0 + hh]],
                closed: true,
            },
        );
        let nx = (w / g).round() as usize;
        for k in 1..nx {
            let x = x0 + k as f64 * g;
            push(
                &mut out,
                Role::Grid,
                SheetGeom::Polyline {
                    points: vec![[x, y0], [x, y0 + hh]],
                    closed: false,
                },
            );
        }
        for k in 0..=nx {
            let x = x0 + k as f64 * g;
            push(
                &mut out,
                Role::GridLabel,
                SheetGeom::Text {
                    at: [x - h, y0 - 1.5 * h],
                    height: h * 0.8,
                    value: fmt_coord(lo[0] + k as f64 * g),
                },
            );
        }
        let ny = (hh / g).round() as usize;
        for k in 1..ny {
            let y = y0 + k as f64 * g;
            push(
                &mut out,
                Role::Grid,
                SheetGeom::Polyline {
                    points: vec![[x0, y], [x0 + w, y]],
                    closed: false,
                },
            );
        }
        for k in 0..=ny {
            let y = y0 + k as f64 * g;
            push(
                &mut out,
                Role::GridLabel,
                SheetGeom::Text {
                    at: [x0 - label_w + h * 0.5, y - h * 0.4],
                    height: h * 0.8,
                    value: fmt_coord(lo[1] + k as f64 * g),
                },
            );
        }
        for l in s.loops {
            push(
                &mut out,
                Role::Section,
                SheetGeom::Polyline {
                    points: l.points_2d.iter().map(|p| map(*p)).collect(),
                    closed: l.closed,
                },
            );
        }
        for l in s.terrain {
            push(
                &mut out,
                Role::Terrain,
                SheetGeom::Polyline {
                    points: l.points_2d.iter().map(|p| map(*p)).collect(),
                    closed: l.closed,
                },
            );
        }
        let (zlo, zhi) = s
            .terrain
            .iter()
            .flat_map(|l| l.points.iter())
            .fold((f64::MAX, f64::MIN), |(a, b), p| (a.min(p[2]), b.max(p[2])));
        if zlo <= zhi {
            push(
                &mut out,
                Role::TerrainLabel,
                SheetGeom::Text {
                    at: [x0 + w * 0.5, y0 + hh + 0.5 * h],
                    height: h * 0.8,
                    value: terrain_label(zlo, zhi),
                },
            );
        }
        if let Some(m) = s.marker {
            let c = map(m);
            let d = h * 0.6;
            push(
                &mut out,
                Role::Marker,
                SheetGeom::Polyline {
                    points: vec![[c[0] - d, c[1]], [c[0] + d, c[1]]],
                    closed: false,
                },
            );
            push(
                &mut out,
                Role::Marker,
                SheetGeom::Polyline {
                    points: vec![[c[0], c[1] - d], [c[0], c[1] + d]],
                    closed: false,
                },
            );
        }
        push(
            &mut out,
            Role::Title,
            SheetGeom::Text {
                at: [x0, y0 + hh + 1.8 * h],
                height: h,
                value: s.title.to_string(),
            },
        );
        let area: f64 = s.loops.iter().map(|l| l.area()).sum();
        push(
            &mut out,
            Role::Area,
            SheetGeom::Text {
                at: [x0, y0 + hh + 0.5 * h],
                height: h * 0.8,
                value: area_label(area),
            },
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square(cx: f64, cy: f64, s: f64) -> SectionLoop {
        let p = vec![[cx - s, cy - s], [cx + s, cy - s], [cx + s, cy + s], [cx - s, cy + s]];
        SectionLoop {
            points: p.iter().map(|q| [q[0], q[1], 0.0]).collect(),
            points_2d: p,
            closed: true,
        }
    }

    #[test]
    fn grid_of_sheets_keeps_true_coordinates() {
        let a = [square(0.0, 4200.0, 2.0)];
        let b = [square(1.0, 4210.0, 2.0)];
        let inputs = [
            SheetInput {
                title: "0+010.00",
                loops: &a,
                marker: Some([0.0, 4200.0]),
                terrain: &[],
            },
            SheetInput {
                title: "0+020.00",
                loops: &b,
                marker: None,
                terrain: &[],
            },
            SheetInput {
                title: "vacía",
                loops: &[],
                marker: None,
                terrain: &[],
            },
        ];
        let cfg = SheetConfig {
            origin: [1000.0, 0.0],
            columns: 2,
            gap: 5.0,
            grid: 2.0,
            text_height: 0.5,
        };
        let items = layout(&inputs, &cfg, &|a| format!("A = {a:.2}"), &|lo, hi| {
            format!("T {lo:.1}-{hi:.1}")
        });
        let of = |sheet, role| items.iter().filter(move |i| i.sheet == sheet && i.role == role);
        // Empty sections get no sheet.
        assert_eq!(items.iter().filter(|i| i.sheet == 2).count(), 0);
        // Area text and elevation labels reflect real values.
        let area = of(0, Role::Area).next().unwrap();
        assert!(matches!(&area.geom, SheetGeom::Text { value, .. } if value == "A = 16.00"));
        let labels: Vec<String> = of(0, Role::GridLabel)
            .filter_map(|i| {
                if let SheetGeom::Text { value, .. } = &i.geom {
                    Some(value.clone())
                } else {
                    None
                }
            })
            .collect();
        assert!(
            labels.contains(&"4196".to_string()) && labels.contains(&"4204".to_string()),
            "{labels:?}"
        );
        // Sheets in the same row do not overlap.
        let frame = |s| match &of(s, Role::Frame).next().unwrap().geom {
            SheetGeom::Polyline { points, .. } => (points[0], points[2]),
            _ => unreachable!(),
        };
        let (f0, f1) = (frame(0), frame(1));
        assert!(f1.0[0] > f0.1[0]);
        assert_eq!(f0.0[1], f1.0[1]);
        assert_eq!(of(0, Role::Marker).count(), 2);
    }

    #[test]
    fn terrain_widens_the_sheet_and_gets_its_elevation_label() {
        let sol = [square(0.0, 4200.0, 2.0)];
        // A terrain profile running 20 m to each side and 30 m above the solid.
        let pts2 = vec![[-20.0, 4228.0], [0.0, 4232.0], [20.0, 4236.0]];
        let terrain = [SectionLoop {
            points: pts2.iter().map(|q| [q[0], 0.0, q[1]]).collect(),
            points_2d: pts2,
            closed: false,
        }];
        let cfg = SheetConfig {
            origin: [0.0, 0.0],
            columns: 1,
            gap: 5.0,
            grid: 2.0,
            text_height: 0.5,
        };
        let with = [SheetInput {
            title: "t",
            loops: &sol,
            marker: None,
            terrain: &terrain,
        }];
        let without = [SheetInput {
            title: "t",
            loops: &sol,
            marker: None,
            terrain: &[],
        }];
        let a = layout(&with, &cfg, &|a| format!("{a}"), &|lo, hi| format!("T {lo:.0}-{hi:.0}"));
        let b = layout(&without, &cfg, &|a| format!("{a}"), &|lo, hi| {
            format!("T {lo:.0}-{hi:.0}")
        });
        let height = |items: &[SheetItem]| match &items.iter().find(|i| i.role == Role::Frame).unwrap().geom {
            SheetGeom::Polyline { points, .. } => points[2][1] - points[0][1],
            _ => unreachable!(),
        };
        assert!(
            height(&a) > height(&b) + 25.0,
            "the frame covers the terrain: {} vs {}",
            height(&a),
            height(&b)
        );
        assert_eq!(a.iter().filter(|i| i.role == Role::Terrain).count(), 1);
        assert!(a.iter().any(|i| matches!(&i.geom, SheetGeom::Text { value, .. } if i.role == Role::TerrainLabel && value == "T 4228-4236")));
        assert!(b
            .iter()
            .all(|i| i.role != Role::Terrain && i.role != Role::TerrainLabel));
    }
}
