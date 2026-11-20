//! The example set in `examples/geotechnics` classifies as its README says.

use std::collections::BTreeMap;

use drillhole_engine::error::TableKind;
use drillhole_engine::table::RawTable;
use geotech_engine::classify::{classify, summarize, ClassifyOptions, Lab};
use geotech_engine::data::{parse_table, GtKind, GtTable};

fn table(name: &str, kind: GtKind) -> GtTable {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/geotechnics")
        .join(name);
    parse_table(
        &RawTable::from_path(&path, TableKind::Geo).unwrap(),
        kind,
        &BTreeMap::new(),
    )
    .unwrap()
}

fn run(name: &str, kind: GtKind, lab: &Lab) -> Vec<geotech_engine::classify::Classification> {
    table(name, kind)
        .records
        .iter()
        .map(|r| classify(r, lab, &ClassifyOptions::default()))
        .collect()
}

#[test]
fn example_tables_classify() {
    let lab = Lab::from_table(&table("lab.csv", GtKind::Lab));
    assert_eq!(lab.0.len(), 3);
    let log = run("logging.csv", GtKind::Logging, &lab);
    let s = summarize(&log);
    assert_eq!(
        (s.total, s.with_rmr, s.with_q, s.with_gsi, s.unclassified, s.invalid),
        (33, 32, 31, 32, 1, 1)
    );
    // GT-01 60-80 has no RQD and no spacing; GT-02 80-100 has Jn = 30 (outside the Q table).
    assert_eq!(log[3].rmr, None);
    assert_eq!((log[12].rmr, log[12].q), (Some(45.0), None));
    // GT-02 40-60: RQD and spacing come from 3.5 fractures/m.
    assert_eq!(log[10].rmr, Some(77.0));
    let classes = |l: &[geotech_engine::classify::Classification]| {
        (1..=5)
            .map(|k| l.iter().filter(|c| c.class == Some(k)).count())
            .collect::<Vec<_>>()
    };
    assert_eq!(classes(&log), vec![9, 15, 8, 0, 0]);
    let map = run("mapping_chainage.csv", GtKind::Mapping, &lab);
    assert_eq!(
        map.iter().map(|c| c.rmr).collect::<Vec<_>>(),
        vec![Some(84.0), Some(70.0), Some(50.0), Some(34.0), Some(81.0)]
    );
    assert!(map.iter().all(|c| c.q.is_some()));
    let co = run("mapping_coords.csv", GtKind::Mapping, &lab);
    assert_eq!(
        co.iter().map(|c| c.class).collect::<Vec<_>>(),
        vec![Some(2), Some(3), Some(4)]
    );
    assert_eq!(table("joints.csv", GtKind::Joints).records.len(), 3);
}
