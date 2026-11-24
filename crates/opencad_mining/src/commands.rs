//! Operations of the drillhole, workings, axes, sections and geology
//! modules (`dh_*`, `ug_*`, `ax_*`, `sc_*`, `gm_*`; see `openmining_ops`).
//! Inputs a user would click can be given as arguments (handles, points);
//! without them the interactive pick starts and its follow-up command
//! writes the result.

use ocs_plugin_api::host::{Handle, HostApi};
use openmining_ops::parse::Args;
use openmining_ops::result::{ErrorCode, OpError, OpResult};
use serde_json::{json, Value};

use crate::cad::CadSink;
use crate::config::ProjectConfig;
use crate::host_sink::{HostSink, OpSink};
use crate::i18n::{fmt, Strings};
use crate::interactive::{
    PickAxisStart, PickEntities, PickKind, PickSelection, PickTracePoint, SectionPick, SelectionKind,
};
use crate::opsrun::Outcome;
use crate::state::{self, Pending};
use crate::{axes, drillholes, geomap, panels, sections, session, solids};

fn failed(msg: impl Into<String>) -> OpError {
    OpError::new(ErrorCode::Failed, msg)
}

/// Turns what an `OpSink` collected into the outcome: the first error is
/// the failure; when the work succeeded the errors are printed as warnings.
fn finish(host: &mut dyn HostApi, ok: bool, errors: Vec<String>, created: Vec<String>, data: Value) -> Outcome {
    if !ok {
        let msg = errors.first().cloned().unwrap_or_else(|| "failed".into());
        return Err(failed(msg));
    }
    for e in &errors {
        host.push_error(e);
    }
    Ok(OpResult {
        data,
        created,
        warnings: errors,
        ..Default::default()
    })
}

fn handle(args: &Args, key: &str) -> Option<Handle> {
    args.get(key)
        .and_then(Value::as_str)
        .and_then(|h| u64::from_str_radix(h, 16).ok())
        .map(Handle::new)
}

fn point(args: &Args, key: &str) -> Option<mining_core::Vec3> {
    let a = args.get(key)?.as_array()?;
    Some([
        a.first()?.as_f64()?,
        a.get(1)?.as_f64()?,
        a.get(2).and_then(Value::as_f64).unwrap_or(0.0),
    ])
}

fn commit(host: &mut dyn HostApi, tab: u64, cfg: &ProjectConfig, label: &str) {
    let mut sink = HostSink::new(host);
    sink.begin(label);
    session::commit(&mut sink, crate::i18n::s(), tab, cfg);
    sink.end();
}

// ───────────────────────────── Drillholes ─────────────────────────────

fn dh_load(host: &mut dyn HostApi, s: &Strings, tab: u64, cfg: &mut ProjectConfig) -> Outcome {
    let result = {
        let mut ask = |t, h: &[String], r: &[_], o: &[_]| session::ask_mapping(&mut *host, s, t, h, r, o);
        drillholes::load(s, cfg, &mut ask)
    };
    let l = result.map_err(|e| {
        session::set_status(host, e.clone());
        let code = if e == s.m_no_collar_file {
            ErrorCode::MissingArgument
        } else {
            ErrorCode::Failed
        };
        OpError::new(code, e)
    })?;
    let (mut errors, mut warnings) = (Vec::new(), Vec::new());
    for (is_err, line) in &l.report {
        if *is_err {
            host.push_error(line);
            errors.push(line.clone());
        } else {
            host.push_info(line);
            warnings.push(line.clone());
        }
    }
    host.push_info(&l.summary);
    let mut sink = HostSink::new(host);
    sink.begin("MIN_DH_LOAD");
    if drillholes::auto_origin(cfg, &l.data) {
        sink.info(&fmt(s.m_origin_set, &[&cfg.origin.x, &cfg.origin.y, &cfg.origin.z]));
    }
    session::commit(&mut sink, s, tab, cfg);
    sink.end();
    let d = &l.data;
    let data = json!({
        "holes": d.collars.len(), "surveys": d.surveys.len(), "assays": d.assays.len(), "geo": d.geo.len(),
        "assay_attributes": l.assay_attrs, "geo_attributes": l.geo_attrs,
        "errors": errors, "warnings": warnings,
        "origin": if cfg.origin.enabled { json!([cfg.origin.x, cfg.origin.y, cfg.origin.z]) } else { Value::Null },
    });
    state::with_tab(tab, |t| t.data = Some(l.data));
    // Channels converted earlier are appended again to the new drillholes.
    let mut data = data;
    if let Some(line) = crate::chcommands::reappend(host, tab, cfg) {
        data["channels"] = json!(line);
    }
    session::push_panels(host, cfg);
    session::set_status(host, l.summary.clone());
    Ok(OpResult::data(data))
}

fn dh_draw(host: &mut dyn HostApi, s: &Strings, tab: u64, cfg: &ProjectConfig) -> Outcome {
    let data = state::with_tab(tab, |t| t.data.clone()).ok_or_else(|| OpError::new(ErrorCode::NoData, s.m_no_data))?;
    let mut sink = OpSink::new(host);
    sink.begin("MIN_DH_DRAW");
    let n = drillholes::draw(&mut sink, s, cfg, &data);
    session::commit(&mut sink, s, tab, cfg);
    sink.end();
    let (errors, created) = (
        std::mem::take(&mut sink.errors),
        std::mem::take(&mut sink.inner.created),
    );
    finish(
        host,
        n > 0,
        errors,
        created,
        json!({ "holes": data.collars.len(), "entities": n }),
    )
}

// ───────────────────────────── Workings ─────────────────────────────

fn run_solid(host: &mut dyn HostApi, s: &Strings, tab: u64, cfg: &ProjectConfig) -> Outcome {
    let Some(p) = state::take_pending() else {
        return Err(OpError::new(ErrorCode::NeedsInput, s.pick_axis));
    };
    let handles: Vec<Handle> = match &p {
        Pending::Axis { axis, profile } => std::iter::once(*axis).chain(*profile).collect(),
        Pending::Boundaries { handles, .. } => handles.to_vec(),
        _ => return Err(OpError::new(ErrorCode::NeedsInput, s.pick_axis)),
    };
    let pts = crate::fetch_points(host, &handles).ok_or_else(|| OpError::new(ErrorCode::NotFound, s.m_not_polyline))?;
    let profile2d: Option<Vec<mining_core::Vec2>> = pts
        .get(1)
        .filter(|_| matches!(p, Pending::Axis { .. }))
        .map(|v| v.iter().map(|q| [q[0], q[1]]).collect());
    let method = match &p {
        Pending::Axis { .. } => solids::Method::Axis {
            axis: &pts[0],
            custom_profile: profile2d.as_deref(),
        },
        Pending::Boundaries { vertical, .. } => solids::Method::Boundaries {
            lines: [&pts[0], &pts[1], &pts[2], &pts[3]],
            vertical: *vertical,
        },
        _ => unreachable!("filtered above"),
    };
    let mut sink = OpSink::new(host);
    sink.begin("MIN_UG_SOLID");
    let built = solids::create(&mut sink, s, &cfg.ug, method);
    if built.is_some() {
        session::commit(&mut sink, s, tab, cfg);
    }
    sink.end();
    let (errors, created) = (
        std::mem::take(&mut sink.errors),
        std::mem::take(&mut sink.inner.created),
    );
    let (volume, sections) = built.unwrap_or((0.0, 0));
    finish(
        host,
        built.is_some(),
        errors,
        created,
        json!({ "volume": (volume * 1000.0).round() / 1000.0, "sections": sections }),
    )
}

fn workings(host: &mut dyn HostApi, s: &Strings, name: &str, args: &Args, tab: u64, cfg: &ProjectConfig) -> Outcome {
    state::with_tab(tab, |t| t.config = cfg.clone());
    let custom = cfg.ug.profile().is_none();
    if name == "ug_axis" {
        let axis = handle(args, "axis").or_else(|| {
            let sel = host.selection();
            (sel.len() == 1 && !custom).then(|| sel[0])
        });
        let profile = handle(args, "profile_polyline");
        if let Some(axis) = axis.filter(|_| !custom || profile.is_some()) {
            state::set_pending(Pending::Axis { axis, profile });
            return run_solid(host, s, tab, cfg);
        }
        let mut prompts = vec![s.pick_axis.to_string()];
        if custom {
            prompts.push(s.pick_profile.to_string());
        }
        host.start_interactive(Box::new(PickEntities::new(
            PickKind::Axis { custom_profile: custom },
            prompts,
        )));
        return Ok(OpResult::waiting(s.pick_axis));
    }
    let vertical = name == "ug_vbound";
    if let Some(list) = crate::gtcommands::handles_arg(args, "lines") {
        let h: [Handle; 4] = list
            .try_into()
            .map_err(|_| OpError::new(ErrorCode::InvalidArgument, "lines needs exactly 4 polylines").at("lines"))?;
        state::set_pending(Pending::Boundaries { vertical, handles: h });
        return run_solid(host, s, tab, cfg);
    }
    let prompts: Vec<String> = if vertical {
        s.pick_boundaries_v.iter()
    } else {
        s.pick_boundaries_h.iter()
    }
    .map(|x| x.to_string())
    .collect();
    let first = prompts[0].clone();
    host.start_interactive(Box::new(PickEntities::new(PickKind::Boundaries { vertical }, prompts)));
    Ok(OpResult::waiting(&first))
}

// ───────────────────────────── Axes ─────────────────────────────

fn axis_data(a: Option<&axes::AxisState>) -> Value {
    match a.and_then(|a| a.design.build().ok().map(|(_, end)| (a, end))) {
        Some((a, end)) => json!({
            "axis": a.id, "elements": a.design.elements.len(), "length": a.design.length(),
            "end": end.point, "end_azimuth": end.azimuth,
        }),
        None => {
            json!({ "axis": Value::Null, "elements": 0, "length": Value::Null, "end": Value::Null, "end_azimuth": Value::Null })
        }
    }
}

fn axis_element(host: &mut dyn HostApi, s: &Strings, name: &str, tab: u64, cfg: &mut ProjectConfig) -> Outcome {
    let element = if name == "ax_line" {
        cfg.ax.straight()
    } else {
        cfg.ax.curve()
    };
    let mut current = state::with_tab(tab, |t| t.axis.clone());
    let mut sink = OpSink::new(host);
    sink.begin(name);
    let end = if name == "ax_undo" {
        axes::undo_last(&mut sink, s, &mut current)
    } else {
        axes::add_element(&mut sink, s, &cfg.ax, &mut current, element)
    };
    if let Some(end) = end.filter(|_| cfg.ax.update_start && name != "ax_undo") {
        axes::set_start(&mut cfg.ax, end);
    } else if let Some(end) = end.filter(|_| name == "ax_undo" && cfg.ax.update_start) {
        axes::set_start(&mut cfg.ax, end);
    }
    session::commit(&mut sink, s, tab, cfg);
    sink.end();
    let (errors, created) = (
        std::mem::take(&mut sink.errors),
        std::mem::take(&mut sink.inner.created),
    );
    let status = current
        .as_ref()
        .map_or_else(|| s.m_axis_empty.to_string(), |a| axes::status(s, a));
    let data = axis_data(current.as_ref());
    let ok = errors.is_empty() || (name == "ax_undo" && end.is_none() && current.is_none());
    state::with_tab(tab, |t| t.axis = current);
    session::push_panel(host, cfg, panels::AX, Some((panels::AX_STATUS, status)));
    finish(host, ok, errors, created, data)
}

/// Start of the axis designer from a pending pick (two points or a polyline).
fn set_start(host: &mut dyn HostApi, s: &Strings, label: &str, tab: u64, cfg: &mut ProjectConfig) -> Outcome {
    let (station, restored) = match state::take_pending() {
        Some(Pending::AxisStart(p, q)) => (
            Some(solids_engine::alignment::Station {
                point: p,
                azimuth: mining_core::azimuth_between([p[0], p[1]], [q[0], q[1]]),
            }),
            None,
        ),
        Some(Pending::AxisFromPolyline(h)) => {
            let ent = host
                .entities_batch(&[h])
                .ok()
                .and_then(|v| v.into_iter().next().flatten());
            match ent
                .as_ref()
                .and_then(|e| crate::host_sink::tag_of(e).and_then(|t| axes::from_tag(&t)))
            {
                Some(a) => (a.design.build().ok().map(|(_, end)| end), Some(a)),
                None => (
                    ent.as_ref()
                        .and_then(crate::polyline_points)
                        .and_then(|p| axes::polyline_end(&p)),
                    None,
                ),
            }
        }
        _ => (None, None),
    };
    let st = station.ok_or_else(|| OpError::new(ErrorCode::NotFound, s.m_not_polyline))?;
    if restored.is_some() {
        host.push_info(s.m_axis_restored);
    }
    axes::set_start(&mut cfg.ax, st);
    host.push_info(&fmt(
        s.m_start_set,
        &[
            &format!("{:.3}", st.point[0]),
            &format!("{:.3}", st.point[1]),
            &format!("{:.3}", st.point[2]),
            &format!("{:.4}", st.azimuth),
        ],
    ));
    let status = restored
        .as_ref()
        .map_or_else(|| s.m_axis_empty.to_string(), |a| axes::status(s, a));
    let elements = restored.as_ref().map_or(0, |a| a.design.elements.len());
    let was_restored = restored.is_some();
    state::with_tab(tab, |t| t.axis = restored);
    commit(host, tab, cfg, label);
    session::push_panel(host, cfg, panels::AX, Some((panels::AX_STATUS, status)));
    Ok(OpResult::data(
        json!({ "start": st.point, "azimuth": st.azimuth, "restored": was_restored, "elements": elements }),
    ))
}

// ───────────────────────────── Sections ─────────────────────────────

fn run_sections(host: &mut dyn HostApi, s: &Strings, tab: u64, cfg: &ProjectConfig) -> Outcome {
    let Some(Pending::Sections { solids, axis, points }) = state::take_pending() else {
        return Err(OpError::new(ErrorCode::NeedsInput, s.pick_solids));
    };
    let ents = host.entities_batch(&solids).unwrap_or_default();
    let meshes = crate::meshes_of(&ents);
    // The terrain chosen in the panel (or by `terrain=`): a Civil TIN surface of the drawing.
    let terrains = if crate::terrain::is_none(&cfg.sc.terrain) {
        Vec::new()
    } else {
        crate::terrain::refresh(host, tab)
    };
    let terrain = if crate::terrain::is_none(&cfg.sc.terrain) {
        None
    } else {
        let t = crate::terrain::t();
        let found = crate::terrain::find(&terrains, &cfg.sc.terrain).ok_or_else(|| {
            let known = terrains.iter().map(|x| x.name.as_str()).collect::<Vec<_>>().join(", ");
            let msg = if terrains.is_empty() {
                t.m_none.to_string()
            } else {
                crate::i18n::fmt(t.m_unknown, &[&cfg.sc.terrain, &known])
            };
            OpError::new(ErrorCode::NotFound, msg).at("terrain")
        })?;
        if found.triangles == 0 {
            return Err(OpError::new(ErrorCode::NoData, crate::i18n::fmt(t.m_empty, &[&found.name])).at("terrain"));
        }
        Some(found)
    };
    let terrain_in = terrain.map(|t| sections::TerrainInput {
        name: &t.name,
        mesh: &t.mesh,
    });
    let axis_pts = match axis {
        Some(h) => Some(
            crate::fetch_points(host, &[h])
                .and_then(|mut v| v.pop())
                .ok_or_else(|| OpError::new(ErrorCode::NotFound, s.m_not_polyline).at("axis"))?,
        ),
        None => None,
    };
    let mut sink = OpSink::new(host);
    sink.begin("MIN_SC_RUN");
    let report = sections::run(
        &mut sink,
        s,
        &cfg.sc,
        &meshes,
        &sections::PlaneInput {
            axis: axis_pts.as_deref(),
            points: &points,
        },
        terrain_in.as_ref(),
    );
    if report.is_some() {
        session::commit(&mut sink, s, tab, cfg);
    }
    sink.end();
    let (errors, created) = (
        std::mem::take(&mut sink.errors),
        std::mem::take(&mut sink.inner.created),
    );
    let data = report.as_ref().map_or(Value::Null, |r| {
        let mut d = json!({ "planes": r.planes, "cut": r.with_hits, "polylines": r.loops });
        if let (Some(tr), Some(t)) = (&r.terrain, terrain) {
            let fin = |x: f64| if x.is_finite() { json!(x) } else { Value::Null };
            d["terrain"] = json!({
                "name": t.name,
                "handle": format!("{:X}", t.handle.value()),
                "planes_cut": tr.planes_cut,
                "polylines": tr.polylines,
                "min_elevation": fin(tr.min_elevation),
                "max_elevation": fin(tr.max_elevation),
                "missed": tr.missed,
            });
        }
        d
    });
    finish(host, report.is_some(), errors, created, data)
}

fn sections_op(host: &mut dyn HostApi, s: &Strings, args: &Args, tab: u64, cfg: &ProjectConfig) -> Outcome {
    state::with_tab(tab, |t| t.config = cfg.clone());
    let solids = crate::gtcommands::handles_arg(args, "solids").unwrap_or_else(|| host.selection());
    let points: Vec<mining_core::Vec3> = ["p1", "p2", "p3"].iter().filter_map(|k| point(args, k)).collect();
    let pick = SectionPick::new(
        cfg.sc.mode,
        solids,
        s.pick_solids.to_string(),
        s.pick_axis.to_string(),
        s.pick_plane_points.map(str::to_string),
    )
    .with_inputs(handle(args, "axis"), points)
    .solids_optional(!crate::terrain::is_none(&cfg.sc.terrain));
    if pick.is_complete() {
        state::set_pending(pick.into_pending());
        return run_sections(host, s, tab, cfg);
    }
    host.start_interactive(Box::new(pick));
    Ok(OpResult::waiting(s.pick_solids))
}

// ───────────────────────────── Geology ─────────────────────────────

fn trace_at(host: &mut dyn HostApi, s: &Strings, tab: u64, cfg: &mut ProjectConfig, p: mining_core::Vec3) -> Outcome {
    (cfg.gm.e, cfg.gm.n, cfg.gm.z) = (p[0], p[1], p[2]);
    let mut sink = OpSink::new(host);
    sink.begin("MIN_GM_TRACE");
    let ok = geomap::insert(&mut sink, s, &cfg.gm, p);
    session::commit(&mut sink, s, tab, cfg);
    sink.end();
    let (errors, created) = (
        std::mem::take(&mut sink.errors),
        std::mem::take(&mut sink.inner.created),
    );
    let data = match geomap::trace_from(&cfg.gm, p) {
        Ok(t) => json!({
            "azimuth": t.azimuth, "bearing": mining_core::azimuth_to_bearing(t.azimuth),
            "point": t.point, "length": t.length,
        }),
        Err(_) => Value::Null,
    };
    session::push_panel(host, cfg, panels::GM, None);
    finish(host, ok, errors, created, data)
}

fn project(host: &mut dyn HostApi, s: &Strings, tab: u64, cfg: &ProjectConfig, handles: &[Handle]) -> Outcome {
    let ents = host.entities_batch(handles).unwrap_or_default();
    let sources: Vec<geomap::Source> = handles
        .iter()
        .zip(&ents)
        .filter_map(|(h, e)| {
            let e = e.as_ref()?;
            let (points, closed) = crate::polyline_geometry(e)?;
            Some(geomap::Source {
                handle: format!("{:X}", h.value()),
                points,
                closed,
                tag: crate::host_sink::tag_of(e),
            })
        })
        .collect();
    let mut sink = OpSink::new(host);
    sink.begin("MIN_GM_PROJECT");
    let n = geomap::project(&mut sink, s, &cfg.gm, &sources);
    if n > 0 {
        session::commit(&mut sink, s, tab, cfg);
    }
    sink.end();
    let (errors, created) = (
        std::mem::take(&mut sink.errors),
        std::mem::take(&mut sink.inner.created),
    );
    finish(
        host,
        n > 0,
        errors,
        created,
        json!({ "projected": n, "skipped": handles.len().saturating_sub(n) }),
    )
}

/// Runs the operation `name` (setting arguments are already in `cfg`).
pub fn run(host: &mut dyn HostApi, name: &str, args: &Args, tab: u64, cfg: &mut ProjectConfig) -> Outcome {
    let s = crate::i18n::s();
    match name {
        "dh_load" => dh_load(host, s, tab, cfg),
        "dh_draw" => dh_draw(host, s, tab, cfg),
        "dh_clear" => Ok(OpResult::data(
            json!({ "removed": drillholes::clear(&mut HostSink::new(host), s) }),
        )),
        "dh_geocolors" => {
            let data =
                state::with_tab(tab, |t| t.data.clone()).ok_or_else(|| OpError::new(ErrorCode::NoData, s.m_no_data))?;
            cfg.dh.geo_colors = drillholes::geo_color_table(&data, &cfg.dh);
            state::with_tab(tab, |t| t.config = cfg.clone());
            session::push_panels(host, cfg);
            Ok(OpResult::data(
                json!({ "values": cfg.dh.geo_colors.len(), "colors": cfg.dh.geo_colors }),
            ))
        }
        "ug_axis" | "ug_hbound" | "ug_vbound" => workings(host, s, name, args, tab, cfg),
        "ax_line" | "ax_curve" | "ax_undo" => axis_element(host, s, name, tab, cfg),
        "ax_new" => {
            state::with_tab(tab, |t| t.axis = None);
            session::push_panel(
                host,
                cfg,
                panels::AX,
                Some((panels::AX_STATUS, s.m_axis_empty.to_string())),
            );
            Ok(OpResult::data(json!({})))
        }
        "ax_pickstart" => match (point(args, "point"), point(args, "toward")) {
            (Some(p), Some(q)) => {
                state::set_pending(Pending::AxisStart(p, q));
                set_start(host, s, "MIN_AX_SETSTART", tab, cfg)
            }
            _ => {
                state::with_tab(tab, |t| t.config = cfg.clone());
                host.start_interactive(Box::new(PickAxisStart::new([
                    s.pick_start_point.to_string(),
                    s.pick_direction.to_string(),
                ])));
                Ok(OpResult::waiting(s.pick_start_point))
            }
        },
        "ax_frompoly" => match handle(args, "polyline") {
            Some(h) => {
                state::set_pending(Pending::AxisFromPolyline(h));
                set_start(host, s, "MIN_AX_SETFROMPOLY", tab, cfg)
            }
            None => {
                state::with_tab(tab, |t| t.config = cfg.clone());
                host.start_interactive(Box::new(PickEntities::new(
                    PickKind::AxisPolyline,
                    vec![s.pick_poly_end.to_string()],
                )));
                Ok(OpResult::waiting(s.pick_poly_end))
            }
        },
        "sc_run" => sections_op(host, s, args, tab, cfg),
        "gm_trace" => {
            let p = [cfg.gm.e, cfg.gm.n, cfg.gm.z];
            trace_at(host, s, tab, cfg, p)
        }
        "gm_trace_pick" => match point(args, "at") {
            Some(p) => trace_at(host, s, tab, cfg, p),
            None => {
                state::with_tab(tab, |t| t.config = cfg.clone());
                host.start_interactive(Box::new(PickTracePoint::new(cfg.gm.clone(), s.pick_point.to_string())));
                Ok(OpResult::waiting(s.pick_point))
            }
        },
        "gm_project" => {
            let handles = crate::gtcommands::handles_arg(args, "traces").unwrap_or_else(|| host.selection());
            if handles.is_empty() {
                state::with_tab(tab, |t| t.config = cfg.clone());
                host.start_interactive(Box::new(PickSelection::new(
                    s.pick_project.to_string(),
                    SelectionKind::Projection,
                )));
                return Ok(OpResult::waiting(s.pick_project));
            }
            project(host, s, tab, cfg, &handles)
        }
        other => Err(OpError::new(
            ErrorCode::UnknownCommand,
            format!("unknown operation {other}"),
        )),
    }
}

/// Follow-ups of the interactive picks: `(operation, outcome)`.
pub fn follow_up(
    host: &mut dyn HostApi,
    command: &str,
    tab: u64,
    cfg: &mut ProjectConfig,
) -> Option<(&'static str, Outcome)> {
    let s = crate::i18n::s();
    match command {
        "MIN_UG_RUN" => {
            let op = match state::peek_pending() {
                Some(Pending::Boundaries { vertical: true, .. }) => "ug_vbound",
                Some(Pending::Boundaries { vertical: false, .. }) => "ug_hbound",
                _ => "ug_axis",
            };
            Some((op, run_solid(host, s, tab, cfg)))
        }
        "MIN_GM_RUN" => match state::take_pending() {
            Some(Pending::TracePoint(p)) => Some(("gm_trace_pick", trace_at(host, s, tab, cfg, p))),
            _ => None,
        },
        "MIN_GM_PROJECT_EXEC" => match state::take_pending() {
            Some(Pending::Projection(h)) => Some(("gm_project", project(host, s, tab, cfg, &h))),
            _ => None,
        },
        "MIN_AX_SETSTART" => Some(("ax_pickstart", set_start(host, s, command, tab, cfg))),
        "MIN_AX_SETFROMPOLY" => Some(("ax_frompoly", set_start(host, s, command, tab, cfg))),
        "MIN_SC_EXEC" => Some(("sc_run", run_sections(host, s, tab, cfg))),
        _ => None,
    }
}
