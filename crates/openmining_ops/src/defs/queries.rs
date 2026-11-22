//! Read-only operations for AI clients: project status, data, statistics,
//! block model sampling and the plugin's entities.

use crate::{input, Group, Kind, OpDef, OpKind, Param, L};

const DH_LIST: &[Param] = &[
    input(
        "hole",
        Kind::Text,
        false,
        L::same("hole"),
        "Hole id: return its intervals (assays and geology) instead of the list of holes.",
    ),
    input(
        "limit",
        Kind::Integer {
            min: Some(1),
            max: Some(10_000),
        },
        false,
        L::same("limit"),
        "Maximum holes or intervals returned (default 500).",
    ),
];

const VARIO: &[Param] = &[input(
    "domain",
    Kind::Index {
        source: "domains",
        all: None,
    },
    false,
    L::same("domain"),
    "Domain (name or index); default the one of the variography settings.",
)];

const SAMPLE: &[Param] = &[
    input(
        "point",
        Kind::Point,
        false,
        L::same("point"),
        "Block containing this point (world x,y,z).",
    ),
    input(
        "from",
        Kind::Point,
        false,
        L::same("from"),
        "Start of a line to sample along (with `to`).",
    ),
    input("to", Kind::Point, false, L::same("to"), "End of the line."),
    input(
        "step",
        Kind::num(Some(0.01), None, 1.0),
        false,
        L::same("step"),
        "Sampling step along the line (default: the smallest block size).",
    ),
    input(
        "columns",
        Kind::Text,
        false,
        L::same("columns"),
        "Comma-separated columns to return (default all).",
    ),
];

const RESOURCES: &[Param] = &[
    input(
        "column",
        Kind::Text,
        false,
        L::same("column"),
        "Grade column (default: OK, else NN/IDW/SK of the estimated variable).",
    ),
    input(
        "cutoffs",
        Kind::Text,
        false,
        L::same("cutoffs"),
        "Comma-separated cut-offs (default: the view cut-off).",
    ),
];

const HELP: &[Param] = &[
    input("op", Kind::Text, false, L::same("op"), "Operation name or command (gt_view, MIN_GT_VIEW): return its parameters, JSON Schema and result. Without it the list of operations."),
    input("group", Kind::Text, false, L::same("group"), "List only one group: project, drillholes, workings, axes, sections, geology, resources, geotechnics, veins."),
    input("workflows", Kind::Bool, false, L::same("workflows"), "Also return the typical workflows (sequences of commands) and the error codes."),
];

const ENTITIES: &[Param] = &[
    input("class", Kind::Text, false, L::same("class"), "Class key filter (DH_TRACE, DH_ASSAY, UG_SOLID, UG_SECTION, GM_TRACE, GM_PROJ, AX_AXIS, SC_CUT, SC_SHEET, RS_COMP, RS_SURFACE, RS_VIEW, GT_REC, VN_SURFACE, VN_PRISM, VN_LONG, VN_BLOCK; comma-separated)."),
    input("layer", Kind::Text, false, L::same("layer"), "Layer filter (exact name)."),
    input("limit", Kind::Integer { min: Some(1), max: Some(10_000) }, false, L::same("limit"), "Maximum entities returned (default 500)."),
    input("offset", Kind::Integer { min: Some(0), max: None }, false, L::same("offset"), "Entities to skip (paging)."),
];

const fn q(
    name: &'static str,
    command: &'static str,
    group: Group,
    summary: &'static str,
    params: &'static [Param],
    result: &'static str,
) -> OpDef {
    OpDef {
        name,
        command,
        aliases: &[],
        label: L::same(name),
        group,
        kind: OpKind::Query,
        summary,
        params,
        positional: &[],
        result,
    }
}

pub static OPS: &[OpDef] = &[
    q(
        "help",
        "MIN_HELP",
        Group::Project,
        "Describe the Open Mining operations: the list (name, command, group, summary), or with `op` its parameters, JSON Schema and result; `workflows=true` adds the typical command sequences and the error codes. Use it to discover what can be done without the guide.",
        HELP,
        "`groups`, `operations` (list of `{name, command, group, kind, summary}`); with `op`: the same plus `aliases`, `positional`, `parameters` (`{key, type, setting, required, help, options}`), `input_schema`, `result`; with `workflows`: `workflows` (text) and `error_codes` (code -> meaning).",
    ),
    q(
        "status",
        "MIN_STATUS",
        Group::Project,
        "What the project holds and what can be done next: drillholes, channels and the narrow-vein model (surfaces, prisms, tonnage), composites, domains and variogram models, block model, contact surfaces, geotechnical tables, axis being designed, drawing path. Call it first, and after an error with code no_data.",
        &[],
        "`drawing` (path or null), `drillholes` (`loaded`, `holes`, `assay_attributes`, `geo_attributes`, `assay_attr`), `composites` (`count`, `variable`, `stale`), `domains` (list of `{name, values, density, model_saved}`), `surfaces`, `block_model` (`blocks`, `columns`) or null, `geotechnics` (`tables`, `classified`), `axis` (current axis or null), `veins` (`{name, channels, surfaces, prisms, tonnage, rules}`: the active vein's progress), `next` (suggested operations with the reason).",
    ),
    q(
        "dh_list",
        "MIN_DH_LIST",
        Group::Drillholes,
        "List the loaded drillholes (collar, depth, interval counts) or, with `hole`, the intervals of one hole with their values.",
        DH_LIST,
        "`holes`: list of `{id, x, y, z, depth, azimuth, dip, assays, geo}`; with `hole`: `{id, assays: [{from, to, values}], geo: [{from, to, values}]}`.",
    ),
    q(
        "rs_stats",
        "MIN_RS_STATS",
        Group::Resources,
        "Statistics of the current composites by domain (count, mean, declustered mean, CV, quantiles), without recompositing.",
        &[],
        "`variable`, `domains`: list of `{name, count, length, mean, declustered_mean, cv, min, p25, median, p75, max}`.",
    ),
    q(
        "rs_variogram",
        "MIN_RS_VARIOGRAM",
        Group::Resources,
        "Experimental variogram points cached for a domain (run rs_vario first) and the working / saved model.",
        VARIO,
        "`domain`, `directions`: list of `{direction, points: [{lag, gamma, pairs}]}`, `working_model`, `saved_model` (null when not saved).",
    ),
    q(
        "bm_info",
        "MIN_BM_INFO",
        Group::Resources,
        "Geometry, domains and columns of the block model (tab or .ombm file).",
        &[],
        "`origin`, `size`, `counts`, `rotation`, `blocks`, `assigned`, `domains` (name -> blocks), `columns` (name -> `{estimated, min, max, mean}`).",
    ),
    q(
        "bm_sample",
        "MIN_BM_SAMPLE",
        Group::Resources,
        "Values of the block model at a point, or along a line every `step` metres (one entry per block crossed).",
        SAMPLE,
        "`samples`: list of `{at, block, ijk, domain, values}` (`block` null outside the model).",
    ),
    q(
        "bm_resources",
        "MIN_BM_RESOURCES",
        Group::Resources,
        "Tonnes, grade, metal and volume by domain above one or more cut-offs on a grade column of the block model.",
        RESOURCES,
        "`column`, `table`: list of `{cutoff, rows: [{domain, blocks, tonnes, grade, metal, volume}]}` (a Total row per cut-off).",
    ),
    q(
        "gt_summary",
        "MIN_GT_SUMMARY",
        Group::Geotechnics,
        "Summary of the last geotechnical classification (same data as gt_classify, without redrawing).",
        &[],
        "as gt_classify (`drawn` is null).",
    ),
    q(
        "entities",
        "MIN_ENTITIES",
        Group::Project,
        "Entities created by Open Mining with their class and attributes (XDATA decoded): volume of solids, area of sections, RMR of geotechnical records, hole/from/to of intervals...",
        ENTITIES,
        "`total` (matching), `entities`: list of `{handle, class, layer, attributes}`, `next_offset` (null at the end).",
    ),
];
