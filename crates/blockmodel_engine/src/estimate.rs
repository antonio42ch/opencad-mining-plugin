//! Estimation of a block model domain by domain.
//!
//! Search passes: the neighbourhood of pass 1 is tried first; blocks that
//! cannot reach `min_samples` retry with the next (larger) pass. The pass
//! that succeeded is stored, which later drives the resource category.

use geostat_engine::kriging::{discretization, estimate, gamma_vv, Method};
use geostat_engine::model::VariogramModel;
use geostat_engine::par_collect;
use geostat_engine::search::{Neighborhood, SearchIndex};
use geostat_engine::Sample;

use crate::model::BlockModel;

#[derive(Debug, Clone, PartialEq)]
pub struct DomainPlan {
    /// 1-based domain id (index in `BlockModel::domain_names` + 1).
    pub id: u8,
    pub model: VariogramModel,
    /// Neighbourhoods in order of use (pass 1, 2, ...).
    pub passes: Vec<Neighborhood>,
    /// Mean for simple kriging.
    pub sk_mean: f64,
    /// Bulk density (t/m3).
    pub density: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Plan {
    /// Variable name, used to name the columns (`au_ok`, `au_ok_var`, ...).
    pub variable: String,
    pub ordinary: bool,
    pub simple: bool,
    pub idw: Option<f64>,
    pub nearest: bool,
    pub discretization: [usize; 3],
    pub domains: Vec<DomainPlan>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct DomainReport {
    pub id: u8,
    pub blocks: usize,
    pub estimated: usize,
    /// Blocks estimated in pass 1, 2, 3...
    pub per_pass: Vec<usize>,
    pub samples: usize,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Report {
    pub domains: Vec<DomainReport>,
}

#[derive(Clone, Copy)]
struct Row {
    ok: Option<geostat_engine::kriging::Estimate>,
    sk: f64,
    idw: f64,
    nn: f64,
    pass: u8,
    n: usize,
    avg_dist: f64,
}

/// Names of the columns written for `variable`.
pub fn column_names(variable: &str) -> Vec<String> {
    [
        "ok", "ok_var", "ok_ke", "ok_slope", "sk", "idw", "nn", "n", "pass", "dist",
    ]
    .iter()
    .map(|s| format!("{variable}_{s}"))
    .collect()
}

/// Estimates every domain in `plan`. `samples` must carry domain ids that
/// match `DomainPlan::id`. Writes the `density` and `tonnage` columns too.
pub fn estimate_model(
    model: &mut BlockModel,
    samples: &[Sample],
    plan: &Plan,
    progress: &(dyn Fn(f32) + Sync),
) -> Report {
    let g = model.geometry;
    let offsets = discretization(g.size, plan.discretization, g.axes());
    let var = plan.variable.clone();
    let n_blocks = model.len();
    for c in column_names(&var) {
        model.column_mut(&c);
    }
    model.column_mut("density");
    model.column_mut("tonnage");
    let mut report = Report::default();
    let nd = plan.domains.len().max(1) as f32;
    for (di, dp) in plan.domains.iter().enumerate() {
        let subset: Vec<Sample> = samples.iter().filter(|s| s.domain == dp.id as u32).cloned().collect();
        let blocks: Vec<usize> = (0..n_blocks).filter(|&i| model.domain[i] == dp.id).collect();
        let mut dr = DomainReport {
            id: dp.id,
            blocks: blocks.len(),
            per_pass: vec![0; dp.passes.len()],
            samples: subset.len(),
            ..Default::default()
        };
        // Tonnage uses the density even where nothing could be estimated.
        for &i in &blocks {
            model.column_mut("density")[i] = dp.density as f32;
            let t = model.inside_volume(i) * dp.density;
            model.column_mut("tonnage")[i] = t as f32;
        }
        if subset.is_empty() || dp.passes.is_empty() {
            report.domains.push(dr);
            continue;
        }
        let cell = dp.passes[0].radii.iter().product::<f64>().cbrt().max(1.0);
        let index = SearchIndex::new(&subset, cell);
        let gvv = gamma_vv(&dp.model, &offsets);
        let rows: Vec<Option<Row>> = par_collect(blocks.len(), |k| {
            let c = g.center(blocks[k]);
            let mut picks = Vec::new();
            for (p, nb) in dp.passes.iter().enumerate() {
                index.query(&subset, c, nb, None, &mut picks);
                if picks.len() >= nb.min_samples.max(1) {
                    let list: Vec<u32> = picks.iter().map(|x| x.0).collect();
                    let run = |m: Method| estimate(m, &dp.model, &subset, &list, c, &offsets, gvv);
                    let ok = if plan.ordinary { run(Method::Ordinary) } else { None };
                    let val = |m: Method| run(m).map_or(f64::NAN, |e| e.value);
                    let first = ok.or_else(|| run(Method::Nearest));
                    return Some(Row {
                        ok,
                        sk: if plan.simple {
                            val(Method::Simple { mean: dp.sk_mean })
                        } else {
                            f64::NAN
                        },
                        idw: plan.idw.map_or(f64::NAN, |pw| val(Method::Idw { power: pw })),
                        nn: if plan.nearest { val(Method::Nearest) } else { f64::NAN },
                        pass: p as u8 + 1,
                        n: list.len(),
                        avg_dist: first.map_or(f64::NAN, |e| e.avg_dist),
                    });
                }
            }
            None
        });
        for (k, row) in rows.into_iter().enumerate() {
            let Some(r) = row else { continue };
            let i = blocks[k];
            dr.estimated += 1;
            dr.per_pass[r.pass as usize - 1] += 1;
            let put = |m: &mut BlockModel, name: &str, v: f64| m.column_mut(&format!("{var}_{name}"))[i] = v as f32;
            if let Some(e) = r.ok {
                put(model, "ok", e.value);
                put(model, "ok_var", e.variance);
                put(model, "ok_ke", e.efficiency);
                put(model, "ok_slope", e.slope);
            }
            put(model, "sk", r.sk);
            put(model, "idw", r.idw);
            put(model, "nn", r.nn);
            put(model, "n", r.n as f64);
            put(model, "pass", r.pass as f64);
            put(model, "dist", r.avg_dist);
        }
        report.domains.push(dr);
        progress((di as f32 + 1.0) / nd);
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::BlockGeometry;
    use crate::region::{assign_domains, Everything};
    use geostat_engine::model::{Kind, Orientation, Structure};

    fn plan(domains: Vec<DomainPlan>) -> Plan {
        Plan {
            variable: "au".into(),
            ordinary: true,
            simple: true,
            idw: Some(2.0),
            nearest: true,
            discretization: [2, 2, 2],
            domains,
        }
    }

    fn domain(id: u8, passes: Vec<Neighborhood>) -> DomainPlan {
        DomainPlan {
            id,
            model: VariogramModel {
                nugget: 0.5,
                structures: vec![Structure {
                    kind: Kind::Spherical,
                    sill: 4.0,
                    ranges: [80.0, 80.0, 80.0],
                }],
                orientation: Orientation::default(),
            },
            passes,
            sk_mean: 3.0,
            density: 2.5,
        }
    }

    fn nb(r: f64, min: usize) -> Neighborhood {
        Neighborhood {
            radii: [r, r, r],
            orientation: Orientation::default(),
            min_samples: min,
            max_samples: 12,
            max_per_octant: 0,
            max_per_hole: 0,
        }
    }

    /// Constant-grade samples on a regular lattice.
    fn lattice(v: f64, domain: u32) -> Vec<Sample> {
        let mut out = Vec::new();
        for i in 0..8 {
            for j in 0..8 {
                for k in 0..4 {
                    out.push(Sample {
                        xyz: [i as f64 * 12.0, j as f64 * 12.0, k as f64 * 12.0],
                        value: v,
                        length: 4.0,
                        hole: (i * 8 + j) as u32,
                        domain,
                    });
                }
            }
        }
        out
    }

    #[test]
    fn constant_field_is_reproduced_by_every_method_and_tonnage_is_set() {
        let mut m = BlockModel::new(BlockGeometry {
            origin: [0.0, 0.0, 0.0],
            size: [10.0, 10.0, 12.0],
            n: [8, 8, 3],
            rotation: 0.0,
        });
        m.domain_names = vec!["Único".into()];
        assign_domains(&mut m, &[&Everything], [2, 2, 2]);
        let rep = estimate_model(
            &mut m,
            &lattice(6.0, 1),
            &plan(vec![domain(1, vec![nb(60.0, 4)])]),
            &|_| {},
        );
        assert_eq!(rep.domains[0].blocks, 8 * 8 * 3);
        assert_eq!(rep.domains[0].estimated, rep.domains[0].blocks);
        for col in ["au_ok", "au_idw", "au_nn"] {
            assert!(m.column(col).unwrap().iter().all(|v| (*v - 6.0).abs() < 1e-4), "{col}");
        }
        // With a wrong mean (3.0) simple kriging does NOT reproduce the data
        // (its weights need not sum to 1); it must still differ from them by a
        // bounded amount. The exact behaviour is covered by the closed-form
        // test in geostat_engine::kriging.
        let sk = m.column("au_sk").unwrap();
        assert!(sk.iter().all(|v| v.is_finite() && (*v - 6.0).abs() < 1.0));
        // With the true mean it reproduces the constant exactly.
        let mut exact = plan(vec![domain(1, vec![nb(60.0, 4)])]);
        exact.domains[0].sk_mean = 6.0;
        estimate_model(&mut m, &lattice(6.0, 1), &exact, &|_| {});
        assert!(m.column("au_sk").unwrap().iter().all(|v| (*v - 6.0).abs() < 1e-4));
        assert!(m.column("au_ok_var").unwrap().iter().all(|v| *v >= 0.0));
        assert!(m.column("au_ok_ke").unwrap().iter().all(|v| v.is_finite()));
        // 10 x 10 x 12 x 2.5 t/m3 = 3000 t per block.
        assert!(m.column("tonnage").unwrap().iter().all(|t| (*t - 3000.0).abs() < 1e-2));
    }

    #[test]
    fn domains_are_estimated_only_from_their_own_samples() {
        let mut m = BlockModel::new(BlockGeometry {
            origin: [0.0, 0.0, 0.0],
            size: [10.0, 10.0, 12.0],
            n: [8, 8, 3],
            rotation: 0.0,
        });
        m.domain_names = vec!["Óxidos".into(), "Sulfuros".into()];
        // Domain 1 = lower half (x < 40), domain 2 = upper half.
        for i in 0..m.len() {
            m.domain[i] = if m.geometry.center(i)[0] < 40.0 { 1 } else { 2 };
            m.fraction[i] = 1.0;
        }
        let mut s = lattice(2.0, 1);
        s.iter_mut().for_each(|p| {
            if p.xyz[0] >= 40.0 {
                p.domain = 2;
                p.value = 9.0;
            }
        });
        estimate_model(
            &mut m,
            &s,
            &plan(vec![domain(1, vec![nb(60.0, 4)]), domain(2, vec![nb(60.0, 4)])]),
            &|_| {},
        );
        let ok = m.column("au_ok").unwrap();
        for (i, (v, d)) in ok.iter().zip(&m.domain).enumerate() {
            let want = if *d == 1 { 2.0 } else { 9.0 };
            assert!((v - want).abs() < 1e-4, "block {i}: {v} vs {want}");
        }
    }

    #[test]
    fn second_pass_reaches_blocks_the_first_cannot() {
        let mut m = BlockModel::new(BlockGeometry {
            origin: [0.0, 0.0, 0.0],
            size: [10.0, 10.0, 12.0],
            n: [30, 2, 1],
            rotation: 0.0,
        });
        m.domain_names = vec!["A".into()];
        assign_domains(&mut m, &[&Everything], [1, 1, 1]);
        // Samples only near x = 0..24: far blocks need the larger radius.
        let s: Vec<Sample> = lattice(5.0, 1).into_iter().filter(|p| p.xyz[0] < 25.0).collect();
        let rep = estimate_model(
            &mut m,
            &s,
            &plan(vec![domain(1, vec![nb(30.0, 4), nb(90.0, 2)])]),
            &|_| {},
        );
        let d = &rep.domains[0];
        assert!(d.per_pass[0] > 0 && d.per_pass[1] > 0, "{d:?}");
        assert!(d.estimated < d.blocks, "the farthest blocks stay unestimated");
        let pass = m.column("au_pass").unwrap();
        let near = m.geometry.index(0, 0, 0);
        let mid = m.geometry.index(10, 0, 0);
        assert_eq!(pass[near], 1.0);
        assert_eq!(pass[mid], 2.0);
    }
}
