//! Prism model of a vein (stage B2): composites of the vein intercepts,
//! in-plane variography of the accumulation, kriging of accumulation and
//! thickness on the vein plane, 3D prisms between the walls, edits on the
//! long section and validation against the channels / holes.

use drillhole_engine::pipeline::HoleModel;
use geostat_engine::crossval::cross_validate;
use geostat_engine::fit::{fit, FitConfig};
use geostat_engine::kriging::{discretization, estimate, gamma_vv, Method};
use geostat_engine::model::{Kind, Orientation, Structure, VariogramModel};
use geostat_engine::search::{Neighborhood, SearchIndex};
use geostat_engine::vario::{experimental, ExpVariogram, VarioParams};
use geostat_engine::{par_collect, Sample};
use mining_core::Vec3;

use crate::contacts::Intercept;
use crate::frame::VeinFrame;
use crate::surfaces::Surfaces;

/// One vein intercept as a sample on the vein plane.
#[derive(Debug, Clone, PartialEq)]
pub struct Composite {
    pub hole: String,
    pub u: f64,
    pub v: f64,
    /// Centre of the intercept (world).
    pub p: Vec3,
    /// True thickness (m): the thickness column when present, otherwise
    /// the intercept length along the vein normal.
    pub thickness: f64,
    /// Thickness-weighted grade of the intercept.
    pub grade: f64,
    /// grade x thickness.
    pub acc: f64,
}

fn weighted(h: &HoleModel, from: f64, to: f64, col: &str) -> Option<f64> {
    let (mut s, mut l) = (0.0, 0.0);
    for a in &h.assays {
        let len = a.to.min(to) - a.from.max(from);
        if len <= 1e-9 {
            continue;
        }
        let val = a
            .attributes
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(col))
            .and_then(|(_, v)| *v);
        if let Some(v) = val.filter(|v| v.is_finite()) {
            s += v * len;
            l += len;
        }
    }
    (l > 0.0).then(|| s / l)
}

/// Composites of the intercepts for the grade `var` (missing grades are
/// skipped and counted). `thick_col` empty = geometric thickness.
pub fn composites(
    holes: &[HoleModel],
    list: &[Intercept],
    frame: &VeinFrame,
    var: &str,
    thick_col: &str,
) -> (Vec<Composite>, usize) {
    let mut out = Vec::new();
    let mut missing = 0;
    for i in list {
        let Some(h) = holes.iter().find(|h| h.collar.hole_id == i.hole) else {
            continue;
        };
        let Some(grade) = weighted(h, i.from, i.to, var) else {
            missing += 1;
            continue;
        };
        let geo = (frame.to_local(i.a)[2] - frame.to_local(i.b)[2]).abs();
        let thickness = if thick_col.trim().is_empty() {
            None
        } else {
            weighted(h, i.from, i.to, thick_col.trim())
        }
        .filter(|t| *t > 0.0)
        .unwrap_or(geo);
        let p = std::array::from_fn(|k| (i.a[k] + i.b[k]) / 2.0);
        let l = frame.to_local(p);
        out.push(Composite {
            hole: i.hole.clone(),
            u: l[0],
            v: l[1],
            p,
            thickness,
            grade,
            acc: grade * thickness,
        });
    }
    (out, missing)
}

fn samples(c: &[Composite], f: impl Fn(&Composite) -> f64) -> Vec<Sample> {
    c.iter()
        .enumerate()
        .map(|(k, x)| Sample {
            xyz: [x.u, x.v, 0.0],
            value: f(x),
            length: 1.0,
            hole: k as u32,
            domain: 1,
        })
        .collect()
}

/// In-plane variograms of the accumulation: omnidirectional, along strike
/// and down dip, and the fitted model (major axis along strike or dip).
#[derive(Debug, Clone, PartialEq)]
pub struct Variography {
    pub omni: ExpVariogram,
    pub strike: ExpVariogram,
    pub dip: ExpVariogram,
    pub model: VariogramModel,
    pub variance: f64,
}

pub fn variography(c: &[Composite], lag: f64, nlags: usize, kind: Kind) -> Variography {
    let s = samples(c, |x| x.acc);
    let n = s.len().max(1) as f64;
    let mean = s.iter().map(|x| x.value).sum::<f64>() / n;
    let variance = (s.iter().map(|x| (x.value - mean).powi(2)).sum::<f64>() / n).max(1e-12);
    let dir = |az: f64| VarioParams {
        azimuth: az,
        az_tol: 22.5,
        ..VarioParams::omni(lag, nlags)
    };
    let omni = experimental(&s, &VarioParams::omni(lag, nlags));
    let (strike, dip) = (experimental(&s, &dir(90.0)), experimental(&s, &dir(0.0)));
    let r0 = lag * nlags as f64 / 2.0;
    let start = |o: Orientation| VariogramModel {
        nugget: 0.1 * variance,
        structures: vec![Structure {
            kind,
            sill: 0.9 * variance,
            ranges: [r0, r0, r0],
        }],
        orientation: o,
    };
    let cfg = |o: Orientation| FitConfig {
        kinds: vec![kind],
        orientation: o,
        fixed_nugget: None,
        max_iter: 300,
    };
    let along = Orientation {
        azimuth: 90.0,
        ..Orientation::default()
    };
    let pairs = |e: &ExpVariogram| e.points.iter().filter(|p| p.pairs > 0).count();
    let mut model = if pairs(&strike) >= 3 && pairs(&dip) >= 3 {
        fit(&[strike.clone(), dip.clone()], &start(along), &cfg(along)).model
    } else {
        let mut m = fit(std::slice::from_ref(&omni), &start(along), &cfg(along)).model;
        for st in m.structures.iter_mut() {
            st.ranges = [st.ranges[0]; 3];
        }
        m
    };
    // 2D data: the axis normal to the plane is not constrained.
    for st in model.structures.iter_mut() {
        st.ranges[2] = st.ranges[0].min(st.ranges[1]);
    }
    Variography {
        omni,
        strike,
        dip,
        model,
        variance,
    }
}

/// Isotropic-in-plane model with ranges along strike and dip.
pub fn plane_model(kind: Kind, nugget: f64, sill: f64, range_strike: f64, range_dip: f64) -> VariogramModel {
    let (rs, rd) = (range_strike.max(0.01), range_dip.max(0.01));
    VariogramModel {
        nugget,
        structures: vec![Structure {
            kind,
            sill,
            ranges: [rs, rd, rs.min(rd)],
        }],
        orientation: Orientation {
            azimuth: 90.0,
            ..Orientation::default()
        },
    }
}

/// Polygon edit on the long section (vein-plane coordinates).
#[derive(Debug, Clone, PartialEq)]
pub enum EditAction {
    Exclude,
    /// Force the grade of the prisms inside.
    Grade(f64),
    /// Force the thickness of the prisms inside (m).
    Thickness(f64),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Edit {
    pub polygon: Vec<[f64; 2]>,
    pub action: EditAction,
}

pub fn inside(poly: &[[f64; 2]], p: [f64; 2]) -> bool {
    let mut c = false;
    let n = poly.len();
    for k in 0..n {
        let (a, b) = (poly[k], poly[(k + n - 1) % n]);
        if (a[1] > p[1]) != (b[1] > p[1]) && p[0] < (b[0] - a[0]) * (p[1] - a[1]) / (b[1] - a[1]) + a[0] {
            c = !c;
        }
    }
    c
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModelParams {
    pub block_u: f64,
    pub block_v: f64,
    pub model: VariogramModel,
    pub radius: f64,
    pub min_samples: usize,
    pub max_samples: usize,
    /// Prisms farther than this from every composite are left out (m).
    pub max_dist: f64,
    /// Discretisation points per side of a prism.
    pub disc: usize,
    pub density: f64,
    pub boundary: Vec<[f64; 2]>,
    pub edits: Vec<Edit>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Prism {
    pub i: usize,
    pub j: usize,
    pub u: f64,
    pub v: f64,
    pub centre: Vec3,
    pub thickness: f64,
    pub area: f64,
    pub volume: f64,
    pub tonnes: f64,
    pub grade: f64,
    pub acc: f64,
    /// Distance to the nearest composite (m).
    pub dist: f64,
    pub n: usize,
    /// Kriging variance of the accumulation.
    pub variance: f64,
    pub edited: bool,
    /// Hanging-wall corners then footwall corners (world).
    pub corners: [Vec3; 8],
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Summary {
    pub prisms: usize,
    pub area: f64,
    pub volume: f64,
    pub tonnes: f64,
    /// Tonnage-weighted grade.
    pub grade: f64,
    pub thickness: f64,
    pub edited: usize,
    /// Cells left out by the boundary, the maximum distance, missing walls, or edits.
    pub out_boundary: usize,
    pub out_distance: usize,
    pub out_walls: usize,
    pub out_edits: usize,
    pub not_estimated: usize,
}

/// Prisms on a regular grid of the vein plane between the two walls.
pub fn estimate_prisms(s: &Surfaces, c: &[Composite], p: &ModelParams) -> (Vec<Prism>, Summary) {
    let acc = samples(c, |x| x.acc);
    let thk = samples(c, |x| x.thickness);
    let g = &s.hw;
    let (u0, v0) = (g.x0, g.y0);
    let (u1, v1) = (g.x0 + (g.nx - 1) as f64 * g.dx, g.y0 + (g.ny - 1) as f64 * g.dy);
    let (bu, bv) = (p.block_u.max(0.1), p.block_v.max(0.1));
    let (ni, nj) = (
        ((u1 - u0) / bu).ceil().max(1.0) as usize,
        ((v1 - v0) / bv).ceil().max(1.0) as usize,
    );
    let nb = Neighborhood {
        radii: [p.radius, p.radius, p.radius],
        orientation: p.model.orientation,
        min_samples: p.min_samples.max(1),
        max_samples: p.max_samples.max(1),
        max_per_octant: 0,
        max_per_hole: 0,
    };
    let index = SearchIndex::new(&acc, p.radius.max(1.0));
    let offsets = discretization(
        [bu, bv, 0.0],
        [p.disc.max(1), p.disc.max(1), 1],
        [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
    );
    let gvv = gamma_vv(&p.model, &offsets);
    let wall = |gs: &geostat_engine::surface::GridSurface, u: f64, v: f64| gs.value_at(u, v);
    // 0 kept, 1 boundary, 2 distance, 3 walls, 4 edit (exclude), 5 not estimated
    let cells: Vec<(u8, Option<Prism>)> = par_collect(ni * nj, |k| {
        let (i, j) = (k % ni, k / ni);
        let (u, v) = (u0 + (i as f64 + 0.5) * bu, v0 + (j as f64 + 0.5) * bv);
        if p.boundary.len() >= 3 && !inside(&p.boundary, [u, v]) {
            return (1, None);
        }
        let dist = c
            .iter()
            .map(|x| ((x.u - u).powi(2) + (x.v - v).powi(2)).sqrt())
            .fold(f64::INFINITY, f64::min);
        if p.max_dist > 0.0 && dist > p.max_dist {
            return (2, None);
        }
        let (h, f) = (wall(&s.hw, u, v), wall(&s.fw, u, v));
        if !(h.is_finite() && f.is_finite()) || h <= f {
            return (3, None);
        }
        let mut edited = false;
        let (mut grade_fix, mut thick_fix) = (None, None);
        for e in &p.edits {
            if e.polygon.len() >= 3 && inside(&e.polygon, [u, v]) {
                match e.action {
                    EditAction::Exclude => return (4, None),
                    EditAction::Grade(x) => grade_fix = Some(x),
                    EditAction::Thickness(x) => thick_fix = Some(x),
                }
                edited = true;
            }
        }
        let centre3 = [u, v, 0.0];
        let mut picks = Vec::new();
        index.query(&acc, centre3, &nb, None, &mut picks);
        let list: Vec<u32> = picks.iter().map(|x| x.0).collect();
        let (ea, et) = if list.len() >= nb.min_samples {
            (
                estimate(Method::Ordinary, &p.model, &acc, &list, centre3, &offsets, gvv),
                estimate(Method::Ordinary, &p.model, &thk, &list, centre3, &offsets, gvv),
            )
        } else {
            (None, None)
        };
        let est = match (ea, et) {
            (Some(a), Some(t)) if t.value > 1e-9 => Some((a.value.max(0.0) / t.value, a.variance, a.n)),
            _ => None,
        };
        let Some((mut grade, variance, n)) = est.or(grade_fix.map(|g| (g, f64::NAN, 0))) else {
            return (5, None);
        };
        if let Some(gf) = grade_fix {
            grade = gf;
        }
        // Corners of the cell on each wall (centre value where a corner is empty).
        let corner = |gs: &geostat_engine::surface::GridSurface, du: f64, dv: f64, at: f64| {
            let z = wall(gs, u + du, v + dv);
            if z.is_finite() {
                z
            } else {
                at
            }
        };
        let d = [(-0.5, -0.5), (0.5, -0.5), (0.5, 0.5), (-0.5, 0.5)];
        let mut hw = [0.0; 4];
        let mut fw = [0.0; 4];
        for (q, (a, b)) in d.iter().enumerate() {
            hw[q] = corner(&s.hw, a * bu, b * bv, h);
            fw[q] = corner(&s.fw, a * bu, b * bv, f).min(hw[q]);
        }
        let mut thickness = hw.iter().zip(&fw).map(|(a, b)| a - b).sum::<f64>() / 4.0;
        if let Some(t) = thick_fix {
            // Centred on the mid-surface.
            for q in 0..4 {
                let m = (hw[q] + fw[q]) / 2.0;
                hw[q] = m + t / 2.0;
                fw[q] = m - t / 2.0;
            }
            thickness = t;
        }
        let corners: [Vec3; 8] = std::array::from_fn(|q| {
            let (a, b) = d[q % 4];
            let w = if q < 4 { hw[q] } else { fw[q - 4] };
            s.frame.to_world([u + a * bu, v + b * bv, w])
        });
        let area = bu * bv;
        let volume = area * thickness;
        let centre = s.frame.to_world([u, v, (h + f) / 2.0]);
        (
            0,
            Some(Prism {
                i,
                j,
                u,
                v,
                centre,
                thickness,
                area,
                volume,
                tonnes: volume * p.density,
                grade,
                acc: grade * thickness,
                dist,
                n,
                variance,
                edited,
                corners,
            }),
        )
    });
    let mut sum = Summary::default();
    let mut out = Vec::new();
    for (code, prism) in cells {
        match code {
            1 => sum.out_boundary += 1,
            2 => sum.out_distance += 1,
            3 => sum.out_walls += 1,
            4 => sum.out_edits += 1,
            5 => sum.not_estimated += 1,
            _ => {}
        }
        if let Some(pr) = prism {
            sum.prisms += 1;
            sum.area += pr.area;
            sum.volume += pr.volume;
            sum.tonnes += pr.tonnes;
            sum.grade += pr.grade * pr.tonnes;
            sum.edited += usize::from(pr.edited);
            out.push(pr);
        }
    }
    if sum.tonnes > 0.0 {
        sum.grade /= sum.tonnes;
    }
    if sum.area > 0.0 {
        sum.thickness = sum.volume / sum.area;
    }
    (out, sum)
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Validation {
    /// Cross-validation of the accumulation (other holes only).
    pub cv_n: usize,
    pub cv_mean_error: f64,
    pub cv_correlation: f64,
    /// Composites against the prism that contains them.
    pub n: usize,
    pub mean_composite: f64,
    pub mean_prism: f64,
    pub correlation: f64,
    /// Composites farthest from their prism grade (largest first, at most 10):
    /// (hole, composite grade, prism grade).
    pub worst: Vec<(String, f64, f64)>,
}

fn corr(a: &[f64], b: &[f64]) -> f64 {
    let n = a.len() as f64;
    if a.len() < 2 {
        return f64::NAN;
    }
    let (ma, mb) = (a.iter().sum::<f64>() / n, b.iter().sum::<f64>() / n);
    let (mut sab, mut saa, mut sbb) = (0.0, 0.0, 0.0);
    for (x, y) in a.iter().zip(b) {
        sab += (x - ma) * (y - mb);
        saa += (x - ma).powi(2);
        sbb += (y - mb).powi(2);
    }
    if saa <= 0.0 || sbb <= 0.0 {
        return f64::NAN;
    }
    sab / (saa * sbb).sqrt()
}

pub fn validate(c: &[Composite], prisms: &[Prism], p: &ModelParams) -> Validation {
    let mut v = Validation::default();
    let acc = samples(c, |x| x.acc);
    let nb = Neighborhood {
        radii: [p.radius; 3],
        orientation: p.model.orientation,
        min_samples: p.min_samples.max(1),
        max_samples: p.max_samples.max(1),
        max_per_octant: 0,
        max_per_hole: 0,
    };
    if acc.len() >= 3 {
        let cv = cross_validate(&acc, &p.model, &nb, Method::Ordinary, 1, true);
        (v.cv_n, v.cv_mean_error, v.cv_correlation) = (cv.stats.n, cv.stats.mean_error, cv.stats.correlation);
    }
    let (bu, bv) = (p.block_u.max(0.1), p.block_v.max(0.1));
    let (mut a, mut b, mut rows) = (Vec::new(), Vec::new(), Vec::new());
    for x in c {
        let hit = prisms
            .iter()
            .find(|pr| (x.u - pr.u).abs() <= bu / 2.0 && (x.v - pr.v).abs() <= bv / 2.0);
        if let Some(pr) = hit {
            a.push(x.grade);
            b.push(pr.grade);
            rows.push((x.hole.clone(), x.grade, pr.grade));
        }
    }
    v.n = a.len();
    if v.n > 0 {
        v.mean_composite = a.iter().sum::<f64>() / v.n as f64;
        v.mean_prism = b.iter().sum::<f64>() / v.n as f64;
    }
    v.correlation = corr(&a, &b);
    rows.sort_by(|x, y| (y.1 - y.2).abs().total_cmp(&(x.1 - x.2).abs()));
    rows.truncate(10);
    v.worst = rows;
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contacts::{Contact, Side};
    use crate::surfaces::{build, Params};

    fn setup() -> (Surfaces, Vec<Composite>) {
        let f = VeinFrame::from_orientation([0.0, 0.0, 3000.0], 75.0, 90.0).unwrap();
        let mut contacts = Vec::new();
        let mut comps = Vec::new();
        for i in 0..11 {
            for j in 0..6 {
                let (u, v) = (i as f64 * 10.0 - 50.0, j as f64 * 10.0 - 25.0);
                let t = 1.0 + 0.01 * u.abs();
                contacts.push(Contact {
                    p: f.to_world([u, v, t / 2.0]),
                    side: Side::Hanging,
                    source: format!("H{i}{j}"),
                });
                contacts.push(Contact {
                    p: f.to_world([u, v, -t / 2.0]),
                    side: Side::Foot,
                    source: format!("F{i}{j}"),
                });
                // Grade rises along strike: 4.5 + 0.05 u.
                let grade = 4.5 + 0.05 * u;
                comps.push(Composite {
                    hole: format!("CH-{i}-{j}"),
                    u,
                    v,
                    p: f.to_world([u, v, 0.0]),
                    thickness: t,
                    grade,
                    acc: grade * t,
                });
            }
        }
        let (s, _) = build(
            &contacts,
            f,
            &Params {
                step: 2.0,
                radius: 30.0,
                ..Default::default()
            },
        )
        .unwrap();
        (s, comps)
    }

    fn params() -> ModelParams {
        ModelParams {
            block_u: 5.0,
            block_v: 5.0,
            model: plane_model(Kind::Spherical, 0.0, 1.0, 40.0, 40.0),
            radius: 30.0,
            min_samples: 2,
            max_samples: 12,
            max_dist: 20.0,
            disc: 2,
            density: 2.7,
            boundary: vec![],
            edits: vec![],
        }
    }

    #[test]
    fn prisms_honour_grades_thickness_and_tonnage() {
        let (s, c) = setup();
        let (prisms, sum) = estimate_prisms(&s, &c, &params());
        assert!(sum.prisms > 100, "{sum:?}");
        // The grade trend is reproduced.
        let at = |u: f64| {
            prisms
                .iter()
                .filter(|p| (p.u - u).abs() < 3.0 && p.v.abs() < 10.0)
                .map(|p| p.grade)
                .next()
                .unwrap()
        };
        assert!(
            (at(-37.5) - (4.5 - 0.05 * 37.5)).abs() < 0.3 && (at(37.5) - (4.5 + 0.05 * 37.5)).abs() < 0.3,
            "{} {}",
            at(-37.5),
            at(37.5)
        );
        // Tonnes = area x thickness x density; thickness near the walls' value.
        let p0 = &prisms[prisms.len() / 2];
        assert!((p0.tonnes - p0.area * p0.thickness * 2.7).abs() < 1e-9);
        assert!((p0.thickness - (1.0 + 0.01 * p0.u.abs())).abs() < 0.15, "{p0:?}");
        // The corners lie on the walls: hanging-wall corners above footwall ones along the normal.
        for q in 0..4 {
            assert!(s.frame.to_local(p0.corners[q])[2] > s.frame.to_local(p0.corners[q + 4])[2]);
        }
        let v = validate(&c, &prisms, &params());
        assert!(
            v.n > 30 && v.correlation > 0.95 && (v.mean_composite - v.mean_prism).abs() < 0.2,
            "{v:?}"
        );
        assert!(v.cv_n > 30 && v.cv_correlation > 0.9, "{v:?}");
    }

    #[test]
    fn boundary_distance_and_edits_limit_the_model() {
        let (s, c) = setup();
        let all = estimate_prisms(&s, &c, &params()).1;
        let mut p = params();
        p.boundary = vec![[-20.0, -20.0], [20.0, -20.0], [20.0, 20.0], [-20.0, 20.0]];
        let (pr, sum) = estimate_prisms(&s, &c, &p);
        assert!(
            sum.prisms < all.prisms && sum.out_boundary > 0 && pr.iter().all(|x| x.u.abs() < 20.0 && x.v.abs() < 20.0)
        );
        p.edits = vec![
            Edit {
                polygon: vec![[-20.0, -20.0], [0.0, -20.0], [0.0, 20.0], [-20.0, 20.0]],
                action: EditAction::Exclude,
            },
            Edit {
                polygon: vec![[0.0, -20.0], [10.0, -20.0], [10.0, 20.0], [0.0, 20.0]],
                action: EditAction::Grade(9.0),
            },
            Edit {
                polygon: vec![[0.0, -20.0], [10.0, -20.0], [10.0, 20.0], [0.0, 20.0]],
                action: EditAction::Thickness(2.0),
            },
        ];
        let (pr, sum) = estimate_prisms(&s, &c, &p);
        assert!(sum.out_edits > 0 && pr.iter().all(|x| x.u > 0.0));
        let fixed: Vec<_> = pr.iter().filter(|x| x.u < 10.0).collect();
        assert!(
            !fixed.is_empty()
                && fixed
                    .iter()
                    .all(|x| x.edited && x.grade == 9.0 && (x.thickness - 2.0).abs() < 1e-9)
        );
        p.boundary.clear();
        p.edits.clear();
        p.max_dist = 1.0;
        let (_, sum) = estimate_prisms(&s, &c, &p);
        assert!(sum.out_distance > 0 && sum.prisms < all.prisms);
    }

    #[test]
    fn in_plane_variography_finds_the_long_axis() {
        // Accumulation varies fast down dip and slowly along strike.
        let f = VeinFrame::from_orientation([0.0; 3], 80.0, 0.0).unwrap();
        let mut c = Vec::new();
        for i in 0..30 {
            for j in 0..30 {
                let (u, v) = (i as f64 * 4.0, j as f64 * 4.0);
                let a = (u / 60.0).sin() + (v / 12.0).sin();
                c.push(Composite {
                    hole: format!("{i}-{j}"),
                    u,
                    v,
                    p: f.to_world([u, v, 0.0]),
                    thickness: 1.0,
                    grade: 3.0 + a,
                    acc: 3.0 + a,
                });
            }
        }
        let vg = variography(&c, 4.0, 15, Kind::Spherical);
        let r = vg.model.structures[0].ranges;
        assert!(r[0] > r[1], "strike range {} should exceed dip range {}", r[0], r[1]);
        assert!(!vg.omni.points.is_empty() && vg.variance > 0.0);
    }

    #[test]
    fn point_in_polygon() {
        let sq = [[0.0, 0.0], [2.0, 0.0], [2.0, 2.0], [0.0, 2.0]];
        assert!(inside(&sq, [1.0, 1.0]) && !inside(&sq, [3.0, 1.0]) && !inside(&sq, [1.0, -0.1]));
    }
}
