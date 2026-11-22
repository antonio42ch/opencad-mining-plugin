//! The documented example (`examples/canales` → `examples/veta`): channels →
//! intercepts → surfaces → in-plane variogram → prisms → blocks between two
//! levels and a raise → categories, all with the plugin's default settings
//! (what `MIN_VN_BUILD` does). It keeps the numbers quoted in
//! `examples/veta/README.md` and `docs/MCP.md` true.

use drillhole_engine::channels::{convert, parse_points, parse_sheet, Options};
use drillhole_engine::model::DataSet;
use drillhole_engine::pipeline::build_holes;
use drillhole_engine::table::RawTable;
use geostat_engine::model::Kind;
use vein_engine::contacts::intercepts;
use vein_engine::frame::fit;
use vein_engine::model::{composites, estimate_prisms, plane_model, validate, variography, ModelParams};
use vein_engine::reserves::{cubicate, totals, BlockParams, Dilution, Rule, Workings};
use vein_engine::surfaces::{build, intercept_contacts, Params};

fn read(f: &str) -> RawTable {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/canales");
    RawTable::from_csv_bytes(&std::fs::read(dir.join(f)).unwrap())
}

#[test]
fn the_channel_example_gives_a_vein_model() {
    let pts = parse_points(&read("puntos_topograficos.csv")).unwrap();
    let sheet = parse_sheet(&read("muestreo_canales.csv"), &Default::default()).unwrap();
    let conv = convert(&sheet, &pts, &Options::default());
    let ds = DataSet {
        collars: conv.collars.clone(),
        surveys: conv.surveys.clone(),
        assays: conv.assays.clone(),
        geo: conv.geo.clone(),
    };
    let holes = build_holes(&ds);
    let ints = intercepts(&holes, "tipo", &["VETA".to_string()]);
    assert_eq!(ints.len(), 24, "one vein intercept per channel");

    // Plane fitted to the intercept ends: steep, dipping to the north-west (the hanging wall is the NW side).
    let ends: Vec<_> = ints.iter().flat_map(|i| [i.a, i.b]).collect();
    let frame = fit(&ends).unwrap();
    let (dip, dipdir) = frame.orientation();
    println!("PLANE {dip:.1}/{dipdir:.1} origin {:?}", frame.origin);
    assert!(
        (60.0..=70.0).contains(&dip) && (290.0..=310.0).contains(&dipdir),
        "{dip} {dipdir}"
    );

    let contacts = intercept_contacts(&frame, &ints);
    let (surf, d) = build(&contacts, frame, &Params::default()).unwrap();
    println!("SURF grid {}x{} nodes {} both {} one_side {} crossings {} thin {} thick {:.3}/{:.3}/{:.3} rms {:.4}/{:.4} unchecked {} worst {:?}", surf.hw.nx, surf.hw.ny, d.nodes, d.both, d.one_side, d.crossings, d.thin, d.thickness_min, d.thickness_mean, d.thickness_max, d.rms_hw, d.rms_fw, d.unchecked, d.worst.first());
    assert_eq!(d.crossings, 0, "{d:?}");
    assert!(d.thickness_mean > 0.3 && d.thickness_mean < 1.5, "{d:?}");

    let (comps, missing) = composites(&holes, &ints, &surf.frame, "au", "potencia");
    assert_eq!((comps.len(), missing), (24, 0));
    let n = comps.len() as f64;
    let (t, a) = (
        comps.iter().map(|c| c.thickness).sum::<f64>(),
        comps.iter().map(|c| c.acc).sum::<f64>(),
    );
    println!(
        "STATS grade_mean {:.3} thickness_mean {:.3} acc_mean {:.3}",
        a / t,
        t / n,
        a / n
    );

    // The in-plane variogram is fitted first (the default of MIN_VN_BUILD).
    let vg = variography(&comps, 10.0, 10, Kind::Spherical);
    let st = vg.model.structures[0];
    println!(
        "MODEL nugget {:.3} sill {:.3} range_strike {:.1} range_dip {:.1} variance {:.2}",
        vg.model.nugget, st.sill, st.ranges[0], st.ranges[1], vg.variance
    );
    let mp = ModelParams {
        block_u: 5.0,
        block_v: 5.0,
        model: plane_model(Kind::Spherical, vg.model.nugget, st.sill, st.ranges[0], st.ranges[1]),
        radius: 60.0,
        min_samples: 2,
        max_samples: 16,
        max_dist: 40.0,
        disc: 3,
        density: 2.7,
        boundary: vec![],
        edits: vec![],
    };
    let (prisms, sum) = estimate_prisms(&surf, &comps, &mp);
    let v = validate(&comps, &prisms, &mp);
    println!("PRISMS {} area {} volume {:.0} tonnes {:.0} grade {:.3} thickness {:.3} out_distance {} out_walls {} not_estimated {}", sum.prisms, sum.area, sum.volume, sum.tonnes, sum.grade, sum.thickness, sum.out_distance, sum.out_walls, sum.not_estimated);
    println!(
        "VALID cv_n {} cv_err {:.3} cv_corr {:.3} n {} mean_comp {:.3} mean_prism {:.3} corr {:.3} worst {:?}",
        v.cv_n,
        v.cv_mean_error,
        v.cv_correlation,
        v.n,
        v.mean_composite,
        v.mean_prism,
        v.correlation,
        v.worst.first()
    );
    assert!(sum.prisms > 200 && sum.tonnes > 10_000.0 && sum.grade > 4.0);
    let (u0, v0) = prisms
        .iter()
        .fold((f64::MAX, f64::MAX), |(x, y), p| (x.min(p.u - 2.5), y.min(p.v - 2.5)));
    let (u1, v1) = prisms
        .iter()
        .fold((f64::MIN, f64::MIN), |(x, y), p| (x.max(p.u + 2.5), y.max(p.v + 2.5)));
    println!("LONG u0 {u0:.1} v0 {v0:.1} width {:.1} height {:.1}", u1 - u0, v1 - v0);

    // Levels along the two galleries (topographic points) and a raise between PT-103 and PT-203.
    let line = |names: &[&str]| -> Vec<[f64; 2]> {
        names
            .iter()
            .map(|n| {
                let q = pts[&n.to_lowercase()];
                let l = surf.frame.to_local([q.x, q.y, q.z]);
                [l[0], l[1]]
            })
            .collect()
    };
    let w = Workings {
        levels: vec![line(&["PT-101", "PT-104"]), line(&["PT-201", "PT-203"])],
        raises: vec![line(&["PT-103", "PT-203"])],
    }
    .sorted();
    let rules = vec![
        Rule {
            category: "Medido".into(),
            min_sides: Some(3),
            max_dist: Some(15.0),
            ..Default::default()
        },
        Rule {
            category: "Indicado".into(),
            min_sides: Some(2),
            max_dist: Some(30.0),
            ..Default::default()
        },
        Rule {
            category: "Inferido".into(),
            min_sides: Some(1),
            ..Default::default()
        },
    ];
    let (blocks, _) = cubicate(
        &prisms,
        &comps,
        &w,
        &Dilution::default(),
        &BlockParams::default(),
        &rules,
    );
    for b in &blocks {
        println!("BLOCK {} {:?} prisms {} area {} thick {:.3} tonnes {:.0} grade {:.3} width {:.3} dil {:.0} @ {:.3} rec {:.0} samples {} dist {:.2} sides {:?} why {:?}", b.name, b.category.map(|c| &rules[c].category), b.prisms, b.area, b.thickness, b.tonnes, b.grade, b.width, b.diluted_tonnes, b.diluted_grade, b.recovered, b.samples, b.dist, b.sides, b.why);
    }
    for (i, t) in totals(&blocks, rules.len()).iter().enumerate() {
        println!(
            "TOTAL {i} blocks {} tonnes {:.0} grade {:.3} dil {:.0} @ {:.3} rec {:.0}",
            t.blocks, t.tonnes, t.grade, t.diluted_tonnes, t.diluted_grade, t.recovered
        );
    }
    // The five blocks and categories quoted in examples/veta/README.md.
    let cat = |n: &str| {
        blocks
            .iter()
            .find(|b| b.name == n)
            .and_then(|b| b.category)
            .map(|c| rules[c].category.as_str())
    };
    assert_eq!(blocks.len(), 5);
    assert_eq!(cat("<L1/<R1"), Some("Inferido"));
    assert_eq!(cat("<L1/>R1"), Some("Inferido"));
    assert_eq!(cat("L1-L2/<R1"), Some("Indicado"));
    assert_eq!(cat("L1-L2/>R1"), Some("Indicado"));
    assert_eq!(cat(">L2/>R1"), Some("Inferido"));
    // The main block has three sampled sides but is 19.6 m from the samples: only the 15 m limit keeps it out of Medido.
    let main = blocks.iter().find(|b| b.name == "L1-L2/>R1").unwrap();
    assert_eq!(main.sampled, 3);
    assert_eq!(main.why, vec!["Medido: dist 19.6 > 15".to_string()]);
    assert!(blocks
        .iter()
        .all(|b| b.diluted_tonnes > b.tonnes && b.recovered < b.diluted_tonnes));
    let all: f64 = blocks.iter().map(|b| b.tonnes + b.pillar_tonnes).sum();
    assert!((all - sum.tonnes).abs() < 1e-6 * sum.tonnes);
}
