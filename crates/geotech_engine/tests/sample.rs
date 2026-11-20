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
