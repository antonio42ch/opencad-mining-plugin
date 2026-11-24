//! One worked call and result per main operation, shown in `docs/MCP.md`.
//! Values come from the documented example (`examples/canales`, `examples/veta`);
//! `…` marks parts left out. Tests check each call names a real operation and
//! each result is valid JSON.

pub struct Example {
    pub op: &'static str,
    pub call: &'static str,
    /// The `data` of the result (abbreviated).
    pub data: &'static str,
}

pub static EXAMPLES: &[Example] = &[
    Example {
        op: "ch_convert",
        call: r#"MIN_CH_CONVERT points="C:\data\puntos_topograficos.csv" sheet="C:\data\muestreo_canales.csv""#,
        data: r#"{"channels": 24, "samples": 72, "holes": 24, "grade_columns": ["AU", "AG"], "text_columns": [], "issues": [], "files": null}"#,
    },
    Example {
        op: "ch_orient",
        call: r#"MIN_CH_ORIENT channel=3950-101-03 p1=0,0,0 p2=0,10,0"#,
        data: r#"{"channel": "3950-101-03", "azimuth": 0.0, "inclination": 0.0, "converted": {"channels": 24, "samples": 72, "issues": [], …}}"#,
    },
    Example {
        op: "vn_surfaces",
        call: r#"MIN_VN_SURFACES vein=EJ1 hw_layer= fw_layer= intercepts=true vein_attr=tipo vein_values=VETA step=2 radius=50"#,
        data: r#"{"vein": "EJ1", "plane": {"dip": 65.9, "dipdir": 299.8, "origin": [500033.26, 8500042.70, 3966.93]},
 "contacts": {"hanging_wall": 24, "footwall": 24, "from_layers": 0, "from_handles": 0, "from_intercepts": 48},
 "grid": {"nx": 74, "ny": 34, "step": 2.0, "nodes": 2516},
 "diagnostics": {"both": 2338, "one_side": 18, "crossings": 0, "thin": 0, "thickness_min": 0.213, "thickness_mean": 0.720, "thickness_max": 1.337, "rms_hw": 0.032, "rms_fw": 0.038, "unchecked": 0},
 "worst": [{"source": "CH-3950-101-02", "side": "hanging_wall", "residual": 0.087, "point": [500002.91, 8500006.40, 3950.0]}, …],
 "file": "C:\\data\\proyecto_veta_EJ1.json"}"#,
    },
    Example {
        op: "vn_variogram",
        call: r#"MIN_VN_VARIOGRAM vein=EJ1 grade=au thickness_col=potencia lag=10 nlags=10 kind=spherical"#,
        data: r#"{"vein": "EJ1", "composites": 24,
 "stats": {"grade_mean": 5.870, "thickness_mean": 0.772, "acc_mean": 4.535, "acc_variance": 28.32},
 "experimental": {"omni": [{"lag": …, "gamma": …, "pairs": …}, …], "strike": […], "dip": […]},
 "model": {"kind": "spherical", "nugget": 12.996, "sill": 21.245, "range_strike": 107.7, "range_dip": 107.7}}"#,
    },
    Example {
        op: "vn_estimate",
        call: r#"MIN_VN_ESTIMATE vein=EJ1 block_u=5 block_v=5 max_distance=40 density=2.7"#,
        data: r#"{"vein": "EJ1",
 "summary": {"prisms": 321, "area": 8025.0, "volume": 5912.0, "tonnes": 15963.0, "grade": 6.818, "thickness": 0.737, "edited": 0, "out_boundary": 0, "out_distance": 69, "out_walls": 30, "out_edits": 0, "not_estimated": 0},
 "validation": {"cv_n": 24, "cv_mean_error": -0.023, "cv_correlation": 0.045, "n": 24, "mean_composite": 5.578, "mean_prism": 6.098, "correlation": 0.602,
   "worst": [{"hole": "CH-4000-201-03", "composite": 18.99, "prism": 7.694}, …]},
 "composites": 24, "file": "C:\\data\\proyecto_veta_EJ1_prismas.csv"}"#,
    },
    Example {
        op: "vn_long",
        call: r#"MIN_VN_LONG vein=EJ1 at=500200,8500000,0 color_by=grade"#,
        data: r#"{"vein": "EJ1", "at": [500200.0, 8500000.0, 0.0], "u0": -88.4, "v0": -23.8, "width": 145.0, "height": 65.0, "prisms": 321, "composites": 24}"#,
    },
    Example {
        op: "vn_edit",
        call: r#"MIN_VN_EDIT action=exclude polygon=1F4"#,
        data: r#"{"vein": "EJ1", "edits": [{"action": "exclude", "value": 0.0, "vertices": 4}], "boundary": 0,
 "estimate": {"summary": {"prisms": …, "out_edits": …, …}, …}}"#,
    },
    Example {
        op: "vn_rules",
        call: r#"MIN_VN_RULES sides=2 dist=20"#,
        data: r#"{"rules": [{"category": "Medido", "min_sides": 3, "max_dist": 15.0, "min_samples": null, "min_grade": null}, {"category": "Indicado", "min_sides": 2, "max_dist": 30.0, …}, {"category": "Inferido", "min_sides": 1, …}],
 "test": {"values": {"sides": 2, "dist": 20.0, "samples": 0, "grade": null}, "rule": 2, "category": "Indicado", "why": ["Medido: sides 2 < 3, dist 20.0 > 15"]}}"#,
    },
    Example {
        op: "vn_cubicate",
        call: r#"MIN_VN_CUBICATE vein=EJ1 level_layer=NIVELES raise_layer=CHIMENEAS mmw=0.8 overbreak_hw=0.1 overbreak_fw=0.05 recovery=90"#,
        data: r#"{"vein": "EJ1", "workings": {"levels": 2, "raises": 1},
 "blocks": [{"name": "L1-L2/>R1", "category": "Indicado", "sides": [true, true, true, null], "sampled": 3, "samples": 18, "dist": 19.62,
             "prisms": 212, "area": 5300.0, "thickness": 0.825, "tonnes": 11801.0, "grade": 6.891, "width": 1.065,
             "diluted_tonnes": 15119.0, "diluted_grade": 5.379, "recovered": 13607.0, "pillar_tonnes": 0.0,
             "why": ["Medido: dist 19.6 > 15"]}, …],
 "totals": [{"category": "Medido", "blocks": 0, …}, {"category": "Indicado", "blocks": 2, "tonnes": 13596.0, "grade": 6.825, "diluted_tonnes": 19137.0, "diluted_grade": 4.849, "recovered": 17223.0}, {"category": "Inferido", "blocks": 3, "tonnes": 2368.0, "grade": 6.777, …}, {"category": "Sin categoría", "blocks": 0, …}],
 "pillars": {"tonnes": 0.0, "grade": null}, "file": "C:\\data\\proyecto_veta_EJ1_bloques.csv"}"#,
    },
    Example {
        op: "vn_build",
        call: r#"MIN_VN_BUILD vein=EJ1 vein_attr=tipo vein_values=VETA grade=au thickness_col=potencia max_distance=40 recovery=90 fit_variogram=true tonnage=true"#,
        data: r#"{"vein": "EJ1", "steps": {"surfaces": {…}, "variogram": {…}, "estimate": {…}, "cubicate": {…}}}"#,
    },
    Example {
        op: "tr_list",
        call: "MIN_TR_LIST",
        data: r#"{"terrains": [{"handle": "754", "name": "Surface 1", "vertices": 9, "triangles": 8, "min": [0.0, 0.0, 100.0], "max": [100.0, 100.0, 105.0], "min_elevation": 100.0, "max_elevation": 105.0}]}"#,
    },
    Example {
        op: "sc_run",
        call: r#"MIN_SC_RUN mode=two_points p1=0,50,0 p2=100,50,0 count=1 terrain="Surface 1""#,
        data: r#"{"planes": 1, "cut": 0, "polylines": 0, "terrain": {"name": "Surface 1", "handle": "754", "planes_cut": 1, "polylines": 1, "min_elevation": 103.0, "max_elevation": 105.0, "missed": []}}"#,
    },
];

pub fn find(op: &str) -> Option<&'static Example> {
    EXAMPLES.iter().find(|e| e.op == op)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn examples_name_real_operations_and_valid_results() {
        for e in EXAMPLES {
            let d = crate::find(e.op).unwrap_or_else(|| panic!("example for unknown operation {}", e.op));
            assert!(
                e.call.starts_with(d.command),
                "{}: call must start with {}",
                e.op,
                d.command
            );
            // Each key=value must be a parameter of the operation.
            let rest = e.call[d.command.len()..].trim();
            let mut i = 0;
            let b: Vec<char> = rest.chars().collect();
            while i < b.len() {
                let start = i;
                while i < b.len() && b[i] != '=' && b[i] != ' ' {
                    i += 1;
                }
                if i < b.len() && b[i] == '=' {
                    let key: String = b[start..i].iter().collect();
                    assert!(d.param(&key).is_some(), "{}: unknown argument {key}", e.op);
                    // Skip the value (quoted or not).
                    i += 1;
                    let mut q = false;
                    while i < b.len() && (q || b[i] != ' ') {
                        if b[i] == '"' {
                            q = !q;
                        }
                        i += 1;
                    }
                }
                while i < b.len() && b[i] == ' ' {
                    i += 1;
                }
            }
            // The abbreviated result must be valid JSON once the `…` marks are removed.
            let json = e.data.replace("…", "null").replace("[null]", "[]");
            let json = json
                .replace(", null}", "}")
                .replace(", null]", "]")
                .replace("{null}", "{}");
            serde_json::from_str::<serde_json::Value>(&json)
                .unwrap_or_else(|er| panic!("{}: result is not JSON ({er}): {json}", e.op));
        }
    }
}
