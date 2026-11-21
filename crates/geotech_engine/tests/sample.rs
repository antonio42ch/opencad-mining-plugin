//! The sample tables used by the end-to-end host test classify as expected.

use std::collections::BTreeMap;

use drillhole_engine::error::TableKind;
use drillhole_engine::table::RawTable;
use geotech_engine::classify::{classify, ClassifyOptions, Lab};
use geotech_engine::data::{parse_table, GtKind};

fn table(name: &str, kind: GtKind) -> geotech_engine::data::GtTable {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/data")
        .join(name);
    parse_table(
        &RawTable::from_path(&path, TableKind::Geo).unwrap(),
        kind,
        &BTreeMap::new(),
    )
    .unwrap()
}

#[test]
fn sample_logging_with_lab() {
    let lab = Lab::from_table(&table("lab.csv", GtKind::Lab));
    let log = table("logging.csv", GtKind::Logging);
    let c: Vec<_> = log
        .records
        .iter()
        .map(|r| classify(r, &lab, &ClassifyOptions::default()))
        .collect();
    // VERT 0-20: 12 (lab 140 MPa) + 17 + 15 + 25 + 15 = 84, class I.
    assert_eq!(c[0].rmr, Some(84.0));
    assert_eq!(c[0].class, Some(1));
    // VERT 40-60: 4 (45 MPa) + 8 + 8 + 12 + 4 = 36, class IV.
    assert_eq!(c[2].rmr, Some(36.0));
    assert_eq!(c[2].class, Some(4));
    assert!(
        c.iter().all(|x| x.rmr.is_some() && x.q.is_some() && x.gsi.is_some()),
        "{c:#?}"
    );
}

#[test]
fn sample_joints_hold_three_sets() {
    use geotech_engine::sets::find_sets;
    use geotech_engine::stereonet::Plane;
    let t = table("joints.csv", GtKind::Joints);
    let poles: Vec<_> = t
        .records
        .iter()
        .filter_map(|r| {
            Some(
                Plane::new(
                    r.num(geotech_engine::data::GtField::Dip)?,
                    r.num(geotech_engine::data::GtField::DipDir)?,
                )
                .pole()
                .vector(),
            )
        })
        .collect();
    assert_eq!(poles.len(), 135);
    let sets = find_sets(&poles, 0, 20.0, 30.0);
    assert_eq!(
        sets.len(),
        3,
        "{:?}",
        sets.iter().map(|s| (s.plane, s.count)).collect::<Vec<_>>()
    );
    let near = |dip: f64, dd: f64| {
        sets.iter()
            .any(|s| (s.plane.dip - dip).abs() < 6.0 && geotech_engine::stereonet::az_diff(s.plane.dipdir, dd) < 12.0)
    };
    assert!(
        near(78.0, 40.0) && near(62.0, 150.0) && near(14.0, 265.0),
        "{:?}",
        sets.iter().map(|s| s.plane).collect::<Vec<_>>()
    );
}
