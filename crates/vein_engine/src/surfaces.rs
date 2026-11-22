//! Hanging-wall and footwall surfaces of a vein, interpolated in the vein
//! frame on a common grid `(u, v)` as the normal offset `w`, and the
//! diagnostics an operator (or an AI assistant) uses to judge and fix them.

use geostat_engine::kriging::{estimate, Method};
use geostat_engine::model::{Kind, Orientation, Structure, VariogramModel};
use geostat_engine::search::{Neighborhood, SearchIndex};
use geostat_engine::surface::{GridSpec, GridSurface};
use geostat_engine::{par_collect, Sample};
use mining_core::Vec3;

use crate::contacts::{Contact, Side};
use crate::frame::VeinFrame;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Interp {
    /// Ordinary kriging, spherical model (range 0 = the search radius).
    Kriging {
        range: f64,
    },
    Idw {
        power: f64,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Params {
    /// Grid spacing on the vein plane (m).
    pub step: f64,
    /// Margin around the contacts (m).
    pub margin: f64,
    /// Search radius: nodes farther from the contacts stay empty (m).
    pub radius: f64,
    pub min_samples: usize,
    pub max_samples: usize,
    pub interp: Interp,
    /// Thickness below which a node is reported as thin (m).
    pub min_thickness: f64,
}

impl Default for Params {
    fn default() -> Self {
        Params {
            step: 2.0,
            margin: 5.0,
            radius: 50.0,
            min_samples: 2,
            max_samples: 12,
            interp: Interp::Kriging { range: 0.0 },
            min_thickness: 0.1,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Surfaces {
    pub frame: VeinFrame,
    /// `w` of the hanging wall and of the footwall on the same grid.
    pub hw: GridSurface,
    pub fw: GridSurface,
}

impl Surfaces {
    /// Thickness `hw - fw` at a grid node (NaN where either is empty).
    pub fn thickness(&self, i: usize, j: usize) -> f64 {
        let k = j * self.hw.nx + i;
        self.hw.z[k] - self.fw.z[k]
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Residual {
    pub source: String,
    pub side: Side,
    pub point: Vec3,
    /// Contact minus surface along the normal (m).
    pub residual: f64,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Diagnostics {
    pub hw_points: usize,
    pub fw_points: usize,
    pub nodes: usize,
    /// Nodes where both walls are defined.
    pub both: usize,
    /// Nodes defined on one wall only (the other is too far from contacts).
    pub one_side: usize,
    /// Nodes where the footwall is above the hanging wall.
    pub crossings: usize,
    /// Nodes thinner than `min_thickness` (crossings excluded).
    pub thin: usize,
    pub thickness_min: f64,
    pub thickness_mean: f64,
    pub thickness_max: f64,
    pub rms_hw: f64,
    pub rms_fw: f64,
    /// Contacts farthest from their surface (largest first, at most 10).
    pub worst: Vec<Residual>,
    /// Contacts outside the grid or next to empty nodes (no residual).
    pub unchecked: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SurfError {
    TooFewContacts { side: Side, found: usize },
    TooManyNodes(usize),
}

impl std::fmt::Display for SurfError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SurfError::TooFewContacts { side, found } => {
                write!(f, "{}: {found} contact point(s), at least 3 are needed", side.key())
            }
            SurfError::TooManyNodes(n) => write!(f, "{n} grid nodes: use a larger step"),
        }
    }
}
impl std::error::Error for SurfError {}

pub const MAX_NODES: usize = 1_000_000;

/// Interpolates `w(u, v)` from local points on `grid`.
pub fn interpolate(points: &[Vec3], grid: &GridSpec, p: &Params) -> GridSurface {
    let samples: Vec<Sample> = points
        .iter()
        .enumerate()
        .map(|(i, q)| Sample {
            xyz: [q[0], q[1], 0.0],
            value: q[2],
            length: 1.0,
            hole: i as u32,
            domain: 1,
        })
        .collect();
    let n = points.len().max(1) as f64;
    let mean = points.iter().map(|q| q[2]).sum::<f64>() / n;
    let var = (points.iter().map(|q| (q[2] - mean).powi(2)).sum::<f64>() / n).max(1e-6);
    let (method, range) = match p.interp {
        Interp::Kriging { range } => (Method::Ordinary, if range > 0.0 { range } else { p.radius }),
        Interp::Idw { power } => (Method::Idw { power }, p.radius),
    };
    let model = VariogramModel {
        nugget: 0.0,
        structures: vec![Structure {
            kind: Kind::Spherical,
            sill: var,
            ranges: [range, range, range.max(1.0)],
        }],
        orientation: Orientation::default(),
    };
    let nb = Neighborhood {
        radii: [p.radius, p.radius, 1.0],
        orientation: Orientation::default(),
        min_samples: p.min_samples.max(1),
        max_samples: p.max_samples.max(1),
        max_per_octant: 0,
        max_per_hole: 0,
    };
    let index = SearchIndex::new(&samples, p.radius.max(1.0));
    let GridSpec { x0, y0, dx, dy, nx, ny } = *grid;
    let z = par_collect(nx * ny, |k| {
        let c = [x0 + (k % nx) as f64 * dx, y0 + (k / nx) as f64 * dy, 0.0];
        let mut picks = Vec::new();
        index.query(&samples, c, &nb, None, &mut picks);
        let list: Vec<u32> = picks.iter().map(|x| x.0).collect();
        if list.len() < nb.min_samples {
            return f64::NAN;
        }
        estimate(method, &model, &samples, &list, c, &[[0.0; 3]], 0.0).map_or(f64::NAN, |e| e.value)
    });
    GridSurface {
        x0,
        y0,
        dx,
        dy,
        nx,
        ny,
        z,
    }
}

/// Builds both surfaces from world contacts in `frame` and diagnoses them.
pub fn build(contacts: &[Contact], frame: VeinFrame, p: &Params) -> Result<(Surfaces, Diagnostics), SurfError> {
    let local: Vec<(Vec3, &Contact)> = contacts.iter().map(|c| (frame.to_local(c.p), c)).collect();
    let side_pts = |s: Side| -> Vec<Vec3> { local.iter().filter(|(_, c)| c.side == s).map(|(l, _)| *l).collect() };
    let (hw, fw) = (side_pts(Side::Hanging), side_pts(Side::Foot));
    for (s, pts) in [(Side::Hanging, &hw), (Side::Foot, &fw)] {
        if pts.len() < 3 {
            return Err(SurfError::TooFewContacts {
                side: s,
                found: pts.len(),
            });
        }
    }
    let (mut lo, mut hi) = ([f64::MAX; 2], [f64::MIN; 2]);
    for (l, _) in &local {
        for a in 0..2 {
            lo[a] = lo[a].min(l[a]);
            hi[a] = hi[a].max(l[a]);
        }
    }
    let step = p.step.max(0.01);
    let x0 = lo[0] - p.margin;
    let y0 = lo[1] - p.margin;
    let nx = (((hi[0] + p.margin) - x0) / step).ceil() as usize + 1;
    let ny = (((hi[1] + p.margin) - y0) / step).ceil() as usize + 1;
    if nx.saturating_mul(ny) > MAX_NODES {
        return Err(SurfError::TooManyNodes(nx * ny));
    }
    let grid = GridSpec {
        x0,
        y0,
        dx: step,
        dy: step,
        nx,
        ny,
    };
    let s = Surfaces {
        frame,
        hw: interpolate(&hw, &grid, p),
        fw: interpolate(&fw, &grid, p),
    };
    let d = diagnose(&s, &local, p);
    Ok((s, d))
}

fn diagnose(s: &Surfaces, local: &[(Vec3, &Contact)], p: &Params) -> Diagnostics {
    let mut d = Diagnostics {
        nodes: s.hw.nx * s.hw.ny,
        ..Default::default()
    };
    let (mut tmin, mut tmax, mut tsum) = (f64::MAX, f64::MIN, 0.0);
    for k in 0..d.nodes {
        let (h, f) = (s.hw.z[k], s.fw.z[k]);
        match (h.is_finite(), f.is_finite()) {
            (true, true) => {
                d.both += 1;
                let t = h - f;
                if t < 0.0 {
                    d.crossings += 1;
                } else if t < p.min_thickness {
                    d.thin += 1;
                }
                tmin = tmin.min(t);
                tmax = tmax.max(t);
                tsum += t;
            }
            (true, false) | (false, true) => d.one_side += 1,
            _ => {}
        }
    }
    if d.both > 0 {
        (d.thickness_min, d.thickness_max, d.thickness_mean) = (tmin, tmax, tsum / d.both as f64);
    }
    let mut res: Vec<Residual> = Vec::new();
    let (mut sq, mut cnt) = ([0.0; 2], [0usize; 2]);
    for (l, c) in local {
        let surf = if c.side == Side::Hanging { &s.hw } else { &s.fw };
        let at = surf.value_at(l[0], l[1]);
        if !at.is_finite() {
            d.unchecked += 1;
            continue;
        }
        let r = l[2] - at;
        let k = usize::from(c.side == Side::Foot);
        sq[k] += r * r;
        cnt[k] += 1;
        res.push(Residual {
            source: c.source.clone(),
            side: c.side,
            point: c.p,
            residual: r,
        });
    }
    d.hw_points = local.iter().filter(|(_, c)| c.side == Side::Hanging).count();
    d.fw_points = local.len() - d.hw_points;
    d.rms_hw = if cnt[0] > 0 {
        (sq[0] / cnt[0] as f64).sqrt()
    } else {
        f64::NAN
    };
    d.rms_fw = if cnt[1] > 0 {
        (sq[1] / cnt[1] as f64).sqrt()
    } else {
        f64::NAN
    };
    res.sort_by(|a, b| b.residual.abs().total_cmp(&a.residual.abs()));
    res.truncate(10);
    d.worst = res;
    d
}

/// Mesh of one wall in world coordinates (quads over defined nodes).
pub fn mesh(s: &Surfaces, side: Side) -> (Vec<Vec3>, Vec<Vec<usize>>) {
    let g = if side == Side::Hanging { &s.hw } else { &s.fw };
    let mut index = vec![usize::MAX; g.nx * g.ny];
    let mut verts = Vec::new();
    for j in 0..g.ny {
        for i in 0..g.nx {
            let k = j * g.nx + i;
            if g.z[k].is_finite() {
                index[k] = verts.len();
                verts.push(s.frame.to_world(g.node(i, j)));
            }
        }
    }
    let mut faces = Vec::new();
    for j in 0..g.ny.saturating_sub(1) {
        for i in 0..g.nx.saturating_sub(1) {
            let q = [
                j * g.nx + i,
                j * g.nx + i + 1,
                (j + 1) * g.nx + i + 1,
                (j + 1) * g.nx + i,
            ];
            if q.iter().all(|k| index[*k] != usize::MAX) {
                faces.push(q.iter().map(|k| index[*k]).collect());
            }
        }
    }
    (verts, faces)
}

/// Splits intercept ends into hanging wall and footwall contacts by their
/// offset along the frame normal.
pub fn intercept_contacts(frame: &VeinFrame, list: &[crate::contacts::Intercept]) -> Vec<Contact> {
    let mut out = Vec::new();
    for i in list {
        let (wa, wb) = (frame.to_local(i.a)[2], frame.to_local(i.b)[2]);
        let (h, f) = if wa >= wb { (i.a, i.b) } else { (i.b, i.a) };
        out.push(Contact {
            p: h,
            side: Side::Hanging,
            source: i.hole.clone(),
        });
        out.push(Contact {
            p: f,
            side: Side::Foot,
            source: i.hole.clone(),
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A vein dipping 70 to the east, 1.5 m thick, with a gentle bulge.
    fn contacts(frame: &VeinFrame) -> Vec<Contact> {
        let mut out = Vec::new();
        for i in 0..9 {
            for j in 0..6 {
                let (u, v) = (i as f64 * 10.0 - 40.0, j as f64 * 8.0 - 20.0);
                let bulge = 0.2 * (u / 40.0) * (v / 20.0);
                out.push(Contact {
                    p: frame.to_world([u, v, 0.75 + bulge]),
                    side: Side::Hanging,
                    source: format!("HW{i}{j}"),
                });
                out.push(Contact {
                    p: frame.to_world([u + 3.0, v + 2.0, -0.75 + bulge]),
                    side: Side::Foot,
                    source: format!("FW{i}{j}"),
                });
            }
        }
        out
    }

    #[test]
    fn surfaces_follow_the_contacts() {
        let f = VeinFrame::from_orientation([1000.0, 2000.0, 3000.0], 70.0, 90.0).unwrap();
        let c = contacts(&f);
        for interp in [Interp::Kriging { range: 0.0 }, Interp::Idw { power: 2.0 }] {
            let p = Params {
                step: 2.0,
                radius: 25.0,
                interp,
                ..Default::default()
            };
            let (s, d) = build(&c, f, &p).unwrap();
            assert_eq!((d.hw_points, d.fw_points), (54, 54));
            assert!(d.both > 0 && d.crossings == 0, "{interp:?} {d:?}");
            assert!(
                (d.thickness_mean - 1.5).abs() < 0.1,
                "{interp:?} mean {}",
                d.thickness_mean
            );
            assert!(
                d.rms_hw < 0.05 && d.rms_fw < 0.05,
                "{interp:?} rms {} {}",
                d.rms_hw,
                d.rms_fw
            );
            let (verts, faces) = mesh(&s, Side::Hanging);
            assert!(!faces.is_empty() && faces.iter().all(|q| q.len() == 4 && q.iter().all(|k| *k < verts.len())));
            // Mesh vertices lie on the hanging wall (w ~ 0.75 near the centre).
            let centre = f.to_local(verts[verts.len() / 2]);
            assert!((centre[2] - 0.75).abs() < 0.25, "{centre:?}");
        }
    }

    #[test]
    fn crossings_far_contacts_and_errors_are_reported() {
        let f = VeinFrame::from_orientation([0.0; 3], 80.0, 0.0).unwrap();
        let mut c = contacts(&f);
        // A footwall contact above the hanging wall makes a crossing and a big residual.
        c.push(Contact {
            p: f.to_world([0.0, 0.0, 3.0]),
            side: Side::Foot,
            source: "BAD".into(),
        });
        let (_, d) = build(
            &c,
            f,
            &Params {
                step: 2.0,
                radius: 25.0,
                interp: Interp::Idw { power: 2.0 },
                ..Default::default()
            },
        )
        .unwrap();
        assert!(d.crossings > 0, "{d:?}");
        assert_eq!(d.worst[0].source, "BAD");
        // Nodes far from contacts stay empty: a small radius leaves holes.
        let (s, _) = build(
            &contacts(&f),
            f,
            &Params {
                step: 1.0,
                radius: 3.0,
                margin: 20.0,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(s.hw.z.iter().any(|z| z.is_nan()));
        let few = vec![Contact {
            p: [0.0; 3],
            side: Side::Hanging,
            source: "a".into(),
        }];
        assert!(matches!(
            build(&few, f, &Params::default()),
            Err(SurfError::TooFewContacts { .. })
        ));
        assert!(matches!(
            build(
                &contacts(&f),
                f,
                &Params {
                    step: 0.01,
                    ..Default::default()
                }
            ),
            Err(SurfError::TooManyNodes(_))
        ));
    }

    #[test]
    fn intercepts_split_by_the_normal() {
        let f = VeinFrame::from_orientation([0.0; 3], 90.0, 90.0).unwrap(); // vertical, normal east
        let i = crate::contacts::Intercept {
            hole: "CH-1".into(),
            from: 0.0,
            to: 1.0,
            a: [-0.5, 0.0, 0.0],
            b: [0.5, 0.0, 0.0],
        };
        let c = intercept_contacts(&f, &[i]);
        assert_eq!((c[0].side, c[0].p[0]), (Side::Hanging, 0.5));
        assert_eq!((c[1].side, c[1].p[0]), (Side::Foot, -0.5));
    }
}
