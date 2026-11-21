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

/// The channel example of `examples/canales` converts completely.
#[test]
fn channel_example_converts() {
    use drillhole_engine::channels::{convert, parse_points, parse_sheet, Options};
    use drillhole_engine::table::RawTable;
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/canales");
    let read = |f: &str| RawTable::from_csv_bytes(&std::fs::read(dir.join(f)).unwrap());
    let pts = parse_points(&read("puntos_topograficos.csv")).unwrap();
    let sheet = parse_sheet(&read("muestreo_canales.csv"), &Default::default()).unwrap();
    assert_eq!(sheet.grade_columns, vec!["AU".to_string(), "AG".to_string()]);
    let c = convert(&sheet, &pts, &Options::default());
    assert!(c.issues.is_empty(), "{:?}", c.issues);
    assert_eq!((c.collars.len(), c.samples), (24, 72));
    // Every channel is horizontal and perpendicular to its N30E gallery (azimuth 120).
    assert!(
        c.collars
            .iter()
            .all(|k| (k.azimuth.unwrap() - 120.0).abs() < 0.2 && k.dip == Some(0.0)),
        "{:?}",
        c.collars[0]
    );
}
