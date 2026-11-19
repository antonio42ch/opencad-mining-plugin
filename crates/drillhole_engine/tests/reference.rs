//! End-to-end check on the reference data set in `tests/data`.

use drillhole_engine::import::{parse_assays, parse_collars, parse_surveys, ColumnMap};
use drillhole_engine::pipeline::build_holes;
use drillhole_engine::table::RawTable;
use drillhole_engine::validate::validate;
use drillhole_engine::DataSet;

const TOL: f64 = 1e-6;

fn load(name: &str) -> RawTable {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/data")
        .join(name);
    RawTable::from_csv_bytes(&std::fs::read(p).unwrap())
}

#[test]
fn reference_holes_match_closed_form() {
    let m = ColumnMap::new();
    let ds = DataSet {
        collars: parse_collars(&load("collar.csv"), &m).unwrap(),
        surveys: parse_surveys(&load("survey.csv"), &m).unwrap(),
        assays: parse_assays(&load("assays.csv"), &m).unwrap(),
        geo: vec![],
    };
    let (clean, msgs) = validate(&ds);
    assert!(msgs.is_empty(), "{msgs:?}");
    let holes = build_holes(&clean);
    let end = |id: &str| {
        holes
            .iter()
            .find(|h| h.collar.hole_id == id)
            .unwrap()
            .trace
            .last()
            .unwrap()
            .xyz
    };
    let s30 = 30f64.to_radians();
    let expected = [
        ("VERT", [500000.0, 8500000.0, 3900.0]),
        (
            "INCL",
            [500100.0 + 200.0 * s30.sin(), 8500000.0, 4000.0 - 200.0 * s30.cos()],
        ),
        ("ARC", [500200.0, 8500100.0, 3900.0]),
    ];
    for (id, e) in expected {
        let g = end(id);
        for i in 0..3 {
            assert!((g[i] - e[i]).abs() < TOL, "{id}: got {g:?}, expected {e:?}");
        }
    }
    let vert = holes.iter().find(|h| h.collar.hole_id == "VERT").unwrap();
    assert_eq!(vert.assays.len(), 4, "3 assays + 1 synthetic gap");
    assert!(vert.assays[2].is_synthetic);
    assert_eq!(vert.assays[0].attributes["au"], Some(0.005));
}
