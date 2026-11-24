//! The MCP guide (`docs/MCP.md`), generated from the definitions.

use std::fmt::Write;

use crate::result::{ErrorCode, PREFIX};
use crate::{Group, Kind, OpDef, OpKind};

pub const WORKFLOWS: &str = r#"## Typical workflows

Start with `MIN_STATUS`: it says what is loaded and lists the next operations (`next`). After an error
with code `no_data`, call it again. Entity handles come from the `created` list of a result, from
`MIN_ENTITIES class=...`, or from the host's own query/selection tools. Search the history for the text
`OPENMINING_RESULT ` (the host may put an icon before it) and parse the JSON after it.

**Drillholes**

```
MIN_DH_LOAD collar="C:\data\collar.csv" survey="C:\data\survey.csv" assays="C:\data\assays.csv"
MIN_DH_LIST
MIN_DH_DRAW assay_attr=cu% scheme=rainbow
```

**Channel samples of the workings** (converted into holes `CH-…` appended to the drillholes)

```
MIN_CH_CONVERT points="C:\data\topo_points.csv" sheet="C:\data\channels.csv"
MIN_DH_DRAW assay_attr=au
MIN_CH_ORIENT channel=3950-101-03 p1=0,0,0 p2=1,1,0
```

**Narrow vein surfaces** (hanging wall and footwall from mapped contacts and vein intercepts)

```
MIN_VN_SURFACES vein=V1 hw_layer=VETA_TECHO fw_layer=VETA_PISO vein_attr=tipo vein_values=VETA step=2 radius=40
```

Read `diagnostics` and `worst`: fix the contacts it names, or change `radius`, `step` or
`method`, and run it again.

**Narrow vein prism model** (after the surfaces; edits are polygons drawn on the long section)

```
MIN_VN_VARIOGRAM vein=V1 grade=au thickness_col=potencia lag=10 nlags=10
MIN_VN_ESTIMATE vein=V1 block_u=5 block_v=5 max_distance=40 density=2.7
MIN_VN_LONG vein=V1
MIN_VN_EDIT action=boundary polygon=<handle>
MIN_VN_EDIT action=exclude polygon=<handle>
```

Check `validation` (cross-validation bias and correlation, composites against their
prism) and the `out_*` counts of `summary` before accepting the model.

**Narrow vein tonnage** (rules are the company's norms: test them before cubicating)

```
MIN_VN_RULES rules=[["Medido",3,15,"",""],["Indicado",2,30,"",""],["Inferido",1,"","",""]]
MIN_VN_RULES sides=2 dist=20
MIN_VN_CUBICATE vein=V1 level_layer=NIVELES raise_layer=CHIMENEAS mmw=0.8 overbreak_hw=0.1 recovery=90
MIN_VN_BUILD vein=V1
```

Each block lists its sampled sides and `why` it missed the earlier rules.

**Whole vein model in one call** (channels already converted; levels and raises are 3D polylines)

```
MIN_VN_BUILD vein=V1 vein_attr=tipo vein_values=VETA grade=au thickness_col=potencia levels=<h1>,<h2> raises=<h3>
```

It accepts every argument of its steps; arguments given are stored, so the next call (or the
panel) keeps them. Read each step's result in `steps` and the diagnostics before reporting.

**Resource estimation** (one variogram model saved per domain before estimating)

```
MIN_RS_COMPOSITE comp_length=2 domains=[["Oxides","OX"],["Sulphides","SU"]]
MIN_RS_STATS
MIN_RS_VARIO domain=Oxides lag=10 nlags=12
MIN_RS_FIT domain=Oxides fit_kind=spherical
MIN_RS_MODELSAVE domain=Oxides
(repeat for every domain)
MIN_RS_ESTIMATE size_x=10 size_y=10 size_z=5 radius1=120 radius2=80 radius3=30
MIN_BM_RESOURCES cutoffs=0.2,0.5,1
MIN_RS_BMVIEW mode=levels attribute=grade_ok
```

Solids as domains: `MIN_RS_SETSOLID target=Oxides solids=1A3F,1A40` (closed meshes).

**Workings and sections**

```
MIN_AX_LINE e=500000 n=8500000 z=3000 bearing=N90E length=100 grade=1
MIN_UG_AXIS axis=<handle of the axis> profile=horseshoe width=4 height=4
MIN_SC_RUN mode=axis axis=<handle> solids=<solid handle> interval=10
```

**Sections with a terrain** (a terrain is a Civil TIN surface of the drawing; `MIN_TR_LIST` names them)

```
MIN_TR_LIST
MIN_SC_RUN mode=axis axis=<handle> solids=<h1>,<h2> interval=10 terrain="Surface 1" terrain_margin=20
MIN_SC_RUN mode=levels z_from=100 z_to=110 z_step=2 terrain="Surface 1"
```

Each vertical section gets the terrain profile (limited to the solids' extent plus the margin),
each level plan its contour; `terrain=` (empty) turns it off. Without solids, only the terrain is cut.

**Geological mapping**

```
MIN_GM_TRACE e=1000 n=2000 z=300 bearing=N45E length=20 dip=60
MIN_GM_PROJECT traces=<handle> proj_mode=delta proj_value=-10
```

**Geotechnics**

```
MIN_GT_LOAD logging="C:\data\logging.csv" lab="C:\data\lab.csv"
MIN_GT_CLASSIFY
MIN_GT_HB_DATA lithology=Andesite
MIN_GT_ESTIMATE method=idw radius1=60 radius2=60 radius3=20
MIN_BM_SAMPLE from=500000,8500000,3100 to=500100,8500000,3100 columns=gt_rmr,gt_class
MIN_GT_VIEW attribute=class
```

"#;

fn kind_detail(k: &Kind) -> String {
    match k {
        Kind::Number { min, max, .. } => match (min, max) {
            (Some(a), Some(b)) => format!("number {a}..{b}"),
            (Some(a), None) => format!("number ≥ {a}"),
            (None, Some(b)) => format!("number ≤ {b}"),
            _ => "number".into(),
        },
        Kind::Integer { min, max } => match (min, max) {
            (Some(a), Some(b)) => format!("integer {a}..{b}"),
            (Some(a), None) => format!("integer ≥ {a}"),
            _ => "integer".into(),
        },
        Kind::Choice { options } => format!(
            "one of `{}`",
            options.iter().map(|o| o.key).collect::<Vec<_>>().join("`, `")
        ),
        Kind::Point => "point `x,y[,z]`".into(),
        Kind::Handle => "entity handle (hex)".into(),
        Kind::Handles => "handles `1A,2B`".into(),
        Kind::Path { .. } => "file path".into(),
        Kind::Name { source } => format!("name among the {source}"),
        Kind::Index { source, .. } => format!("name or index among the {source}"),
        Kind::Table { columns, .. } => format!(
            "table `{}`",
            columns.iter().map(|c| c.en).collect::<Vec<_>>().join(" | ")
        ),
        other => other.type_name().into(),
    }
}

fn op(out: &mut String, d: &OpDef) {
    let _ = writeln!(out, "### `{}` — `{}`\n", d.name, d.command);
    let _ = writeln!(
        out,
        "{}{}\n",
        if d.kind == OpKind::Query { "*Read-only.* " } else { "" },
        d.summary
    );
    if !d.aliases.is_empty() {
        let _ = writeln!(out, "Old names: `{}`.\n", d.aliases.join("`, `"));
    }
    if d.params.is_empty() {
        let _ = writeln!(out, "No arguments.\n");
    } else {
        let _ = writeln!(out, "| Argument | Type | Setting | Description |\n|---|---|---|---|");
        for p in d.params {
            let set = if p.setting.is_some() {
                "yes"
            } else if p.required {
                "**required**"
            } else {
                ""
            };
            let _ = writeln!(
                out,
                "| `{}` | {} | {} | {} |",
                p.key,
                kind_detail(&p.kind),
                set,
                p.help.replace('|', "\\|")
            );
        }
        out.push('\n');
    }
    if !d.positional.is_empty() {
        let _ = writeln!(
            out,
            "Positional form: `{} {}` (`-` skips one).\n",
            d.command,
            d.positional.join(" ")
        );
    }
    let _ = writeln!(out, "Result `data`: {}\n", d.result);
    if let Some(e) = crate::examples::find(d.name) {
        let _ = writeln!(
            out,
            "Example (values abbreviated with `…`):\n\n```\n{}\n→ {}\n```\n",
            e.call, e.data
        );
    }
}

pub fn markdown() -> String {
    let mut out = String::new();
    out.push_str("<!-- Generated from crates/openmining_ops (cargo run -p openmining_cli -- mcp-doc). Do not edit by hand. -->\n");
    out.push_str("# Open Mining — operations for AI clients (MCP)\n\n");
    out.push_str(
        "Open CAD Studio exposes the drawing to AI clients with `OpenCADStudio --mcp`. Every Open Mining \
action below is a command run with the `run` tool (`ocs_run`), with arguments as `key=value` \
(quote values with spaces: `path=\"C:\\Mis datos\\log.csv\"`). Arguments marked *setting* are project \
settings: omitted, the current value is used (the one in the panel); given, it is stored for later \
runs. Entity inputs that a user would click (axes, solids, points) are passed as handles or coordinates.\n\n",
    );
    let _ = writeln!(
        out,
        "Each command writes its usual messages and **one result line** to the command history (read it with \
the history tool):\n\n```\n{PREFIX}{{\"op\":\"gt_load\",\"ok\":true,\"status\":\"done\",\"data\":{{…}},\"created\":[\"1A3F\"],\"warnings\":[]}}\n{PREFIX}{{\"op\":\"gt_hb\",\"ok\":false,\"error\":{{\"code\":\"out_of_range\",\"message\":\"…\",\"param\":\"gsi\"}}}}\n```\n"
    );
    out.push_str("`status` is `waiting_input` when a pick was started because an input was missing; pass it as an argument instead.\n\n");
    out.push_str("## Error codes\n\n| Code | Meaning |\n|---|---|\n");
    for c in ErrorCode::ALL {
        let _ = writeln!(out, "| `{}` | {} |", c.as_str(), c.describe());
    }
    out.push('\n');
    out.push_str(WORKFLOWS);
    let groups = [
        Group::Project,
        Group::Drillholes,
        Group::Workings,
        Group::Axes,
        Group::Sections,
        Group::Geology,
        Group::Resources,
        Group::Geotechnics,
        Group::Veins,
    ];
    for g in groups {
        let ops: Vec<&OpDef> = crate::all().into_iter().filter(|d| d.group == g).collect();
        if ops.is_empty() {
            continue;
        }
        let _ = writeln!(out, "## {}\n", g.title());
        for d in ops {
            op(&mut out, d);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn guide_is_up_to_date() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/MCP.md");
        let now = super::markdown();
        match std::fs::read_to_string(path) {
            // Windows checkouts may turn LF into CRLF.
            Ok(saved) if saved.replace("\r\n", "\n") == now => {}
            _ if std::env::var_os("OPENMINING_BLESS").is_some() => std::fs::write(path, now).unwrap(),
            _ => panic!("docs/MCP.md is stale: run `OPENMINING_BLESS=1 cargo test -p openmining_ops`"),
        }
    }
}
