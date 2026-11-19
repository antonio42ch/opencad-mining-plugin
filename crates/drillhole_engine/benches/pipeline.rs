//! `cargo bench -p drillhole_engine` — throughput of validate + desurvey +
//! mapping on a synthetic project (1 000 holes x 20 surveys x 200 assays).

use std::collections::BTreeMap;

use criterion::{criterion_group, criterion_main, Criterion};
use drillhole_engine::{pipeline::build_holes, validate::validate, Assay, Collar, DataSet, Survey};

fn synthetic(holes: usize) -> DataSet {
    let mut ds = DataSet::default();
    for h in 0..holes {
        let id = format!("DH{h:05}");
        ds.collars.push(Collar {
            hole_id: id.clone(),
            x: 500_000.0 + h as f64 * 25.0,
            y: 8_500_000.0,
            z: 4_000.0,
            max_depth: 400.0,
            azimuth: Some(90.0),
            dip: Some(-60.0),
        });
        for s in 0..20 {
            ds.surveys.push(Survey {
                hole_id: id.clone(),
                depth: s as f64 * 20.0,
                azimuth: 90.0 + s as f64 * 0.5,
                dip: -60.0 + s as f64 * 0.3,
            });
        }
        for a in 0..200 {
            ds.assays.push(Assay {
                hole_id: id.clone(),
                from: a as f64 * 2.0,
                to: a as f64 * 2.0 + 2.0,
                attributes: BTreeMap::from([("cu".into(), Some((a % 37) as f64 / 10.0))]),
            });
        }
    }
    ds
}

fn bench(c: &mut Criterion) {
    let ds = synthetic(1_000);
    c.bench_function("validate 1k holes", |b| b.iter(|| validate(&ds)));
    let (clean, _) = validate(&ds);
    c.bench_function("build_holes 1k holes", |b| b.iter(|| build_holes(&clean)));
}

criterion_group! { name = benches; config = Criterion::default().sample_size(10); targets = bench }
criterion_main!(benches);
