//! `cargo bench -p blockmodel_engine` — estimation at the scale of the
//! reference study (114 holes, ~3 000 4 m composites, 10x10x12 m blocks over
//! 630 x 570 x 540 m) and of a 1 M block model.

use blockmodel_engine::estimate::{estimate_model, DomainPlan, Plan};
use blockmodel_engine::region::{assign_domains, Everything};
use blockmodel_engine::{BlockGeometry, BlockModel};
use criterion::{criterion_group, criterion_main, Criterion};
use geostat_engine::model::{Kind, Orientation, Structure, VariogramModel};
use geostat_engine::search::Neighborhood;
use geostat_engine::Sample;

fn samples() -> Vec<Sample> {
    let mut v = Vec::new();
    let mut h = 0u32;
    for i in 0..12 {
        for j in 0..10 {
            for k in 0..27 {
                let (x, y, z) = (
                    96_440.0 + i as f64 * 55.0,
                    87_550.0 + j as f64 * 57.0,
                    1_710.0 + k as f64 * 20.0,
                );
                v.push(Sample {
                    xyz: [x, y, z],
                    value: 2.0 + ((x / 90.0).sin() + (y / 70.0).cos() + (z / 40.0).sin()).abs() * 2.0,
                    length: 4.0,
                    hole: h,
                    domain: 1,
                });
            }
            h += 1;
        }
    }
    v
}

fn plan() -> Plan {
    let o = Orientation {
        azimuth: 325.0,
        dip: 0.0,
        rake: 0.0,
    };
    let nb = Neighborhood {
        radii: [200.0, 150.0, 50.0],
        orientation: o,
        min_samples: 4,
        max_samples: 8,
        max_per_octant: 1,
        max_per_hole: 0,
    };
    Plan {
        variable: "au".into(),
        ordinary: true,
        simple: false,
        idw: None,
        nearest: false,
        discretization: [4, 4, 4],
        domains: vec![DomainPlan {
            id: 1,
            model: VariogramModel {
                nugget: 3.0,
                structures: vec![Structure {
                    kind: Kind::Spherical,
                    sill: 14.0,
                    ranges: [150.0, 110.0, 40.0],
                }],
                orientation: o,
            },
            passes: vec![nb.clone(), nb.scaled(1.5)],
            sk_mean: 4.0,
            density: 2.7,
        }],
    }
}

fn bench(c: &mut Criterion) {
    let s = samples();
    let mut g = c.benchmark_group("estimate");
    g.sample_size(10);
    for (name, n) in [
        ("study 63x57x46 = 165k blocks", [63, 57, 46]),
        ("1M blocks 100x100x100", [100, 100, 100]),
    ] {
        g.bench_function(name, |b| {
            b.iter(|| {
                let mut m = BlockModel::new(BlockGeometry {
                    origin: [96_440.0, 87_550.0, 1_704.0],
                    size: [10.0, 10.0, 12.0],
                    n,
                    rotation: 0.0,
                });
                m.domain_names = vec!["Único".into()];
                assign_domains(&mut m, &[&Everything], [4, 4, 4]);
                estimate_model(&mut m, &s, &plan(), &|_| {})
            })
        });
    }
    g.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
