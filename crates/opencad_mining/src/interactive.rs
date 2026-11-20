//! Interactive picks. They carry no `HostApi`, so when the input is complete
//! they park it in `state::Pending` and dispatch a follow-up command that
//! does the work with full host access.

use mining_core::Vec3;
use ocs_plugin_api::host::{CommandStep, Handle, InteractiveCommand, PreviewWire};

use crate::config::GeoMapConfig;
use crate::state::{set_pending, Pending};

pub enum PickKind {
    Axis {
        custom_profile: bool,
    },
    Boundaries {
        vertical: bool,
    },
    /// Polyline whose end continues an axis.
    AxisPolyline,
    /// Axis polyline for block-model sections.
    ViewAxis,
}

pub struct PickEntities {
    prompts: Vec<String>,
    picked: Vec<Handle>,
    kind: PickKind,
}

impl PickEntities {
    pub fn new(kind: PickKind, prompts: Vec<String>) -> Self {
        PickEntities {
            prompts,
            picked: Vec::new(),
            kind,
        }
    }
}

impl InteractiveCommand for PickEntities {
    fn prompt(&self) -> String {
        self.prompts.get(self.picked.len()).cloned().unwrap_or_default()
    }
    fn on_point(&mut self, _pt: [f64; 3]) -> CommandStep {
        CommandStep::NeedPoint
    }
    fn needs_object_pick(&self) -> bool {
        true
    }
    fn on_object_pick(&mut self, handle: Handle, _pt: [f64; 3]) -> CommandStep {
        self.picked.push(handle);
        if self.picked.len() < self.prompts.len() {
            return CommandStep::NeedPoint;
        }
        let p = &self.picked;
        let (pending, next) = match self.kind {
            PickKind::Axis { custom_profile } => (
                Pending::Axis {
                    axis: p[0],
                    profile: custom_profile.then(|| p[1]),
                },
                "MIN_UG_RUN",
            ),
            PickKind::Boundaries { vertical } => (
                Pending::Boundaries {
                    vertical,
                    handles: [p[0], p[1], p[2], p[3]],
                },
                "MIN_UG_RUN",
            ),
            PickKind::AxisPolyline => (Pending::AxisFromPolyline(p[0]), "MIN_AX_SETFROMPOLY"),
            PickKind::ViewAxis => (
                Pending::BmView {
                    axis: Some(p[0]),
                    points: Vec::new(),
                },
                "MIN_RS_BMVIEW_EXEC",
            ),
        };
        set_pending(pending);
        CommandStep::Dispatch(next.into())
    }
    // TODO(API >= 24): implement `on_undo_step` (pop the last pick on
    // Ctrl+Z) once a host release ships plugin API 24.
}

pub struct PickTracePoint {
    cfg: GeoMapConfig,
    prompt: String,
}

impl PickTracePoint {
    pub fn new(cfg: GeoMapConfig, prompt: String) -> Self {
        PickTracePoint { cfg, prompt }
    }
}

impl InteractiveCommand for PickTracePoint {
    fn prompt(&self) -> String {
        self.prompt.clone()
    }
    fn on_point(&mut self, pt: [f64; 3]) -> CommandStep {
        set_pending(Pending::TracePoint(pt));
        CommandStep::Dispatch("MIN_GM_RUN".into())
    }
    fn on_cursor_move(&mut self, pt: [f64; 3]) -> Vec<PreviewWire> {
        match crate::geomap::trace_from(&self.cfg, pt) {
            Ok(t) => {
                let (a, b) = t.endpoints();
                vec![PreviewWire::Polyline {
                    points: vec![a, b],
                    color: Some([1.0, 0.25, 0.25, 1.0]),
                }]
            }
            Err(_) => Vec::new(),
        }
    }
}

/// Two points: axis start and a point giving its direction (with a preview line).
pub struct PickAxisStart {
    prompts: [String; 2],
    first: Option<Vec3>,
}

impl PickAxisStart {
    pub fn new(prompts: [String; 2]) -> Self {
        PickAxisStart { prompts, first: None }
    }
}

impl InteractiveCommand for PickAxisStart {
    fn prompt(&self) -> String {
        self.prompts[usize::from(self.first.is_some())].clone()
    }
    fn on_point(&mut self, pt: [f64; 3]) -> CommandStep {
        match self.first {
            None => {
                self.first = Some(pt);
                CommandStep::NeedPoint
            }
            Some(p) => {
                set_pending(Pending::AxisStart(p, pt));
                CommandStep::Dispatch("MIN_AX_SETSTART".into())
            }
        }
    }
    fn on_cursor_move(&mut self, pt: [f64; 3]) -> Vec<PreviewWire> {
        self.first.map_or_else(Vec::new, |p| {
            vec![PreviewWire::Polyline {
                points: vec![p, pt],
                color: Some([1.0, 0.3, 1.0, 1.0]),
            }]
        })
    }
}

/// Section run: solids (unless pre-selected), then the inputs of the plane mode.
pub struct SectionPick {
    mode: usize,
    solids: Vec<Handle>,
    axis: Option<Handle>,
    points: Vec<Vec3>,
    prompt_solids: String,
    prompt_axis: String,
    prompt_points: [String; 3],
}

impl SectionPick {
    pub fn new(
        mode: usize,
        solids: Vec<Handle>,
        prompt_solids: String,
        prompt_axis: String,
        prompt_points: [String; 3],
    ) -> Self {
        SectionPick {
            mode,
            solids,
            axis: None,
            points: Vec::new(),
            prompt_solids,
            prompt_axis,
            prompt_points,
        }
    }

    fn points_needed(&self) -> usize {
        match self.mode {
            1 => 2,
            3 => 3,
            _ => 0,
        }
    }

    pub fn is_complete(&self) -> bool {
        !self.solids.is_empty() && (self.mode != 0 || self.axis.is_some()) && self.points.len() >= self.points_needed()
    }

    pub fn into_pending(self) -> Pending {
        Pending::Sections {
            solids: self.solids,
            axis: self.axis,
            points: self.points,
        }
    }

    fn step(&mut self) -> CommandStep {
        if self.is_complete() {
            set_pending(Pending::Sections {
                solids: self.solids.clone(),
                axis: self.axis,
                points: self.points.clone(),
            });
            CommandStep::Dispatch("MIN_SC_EXEC".into())
        } else {
            CommandStep::NeedPoint
        }
    }
}

impl InteractiveCommand for SectionPick {
    fn prompt(&self) -> String {
        if self.solids.is_empty() {
            self.prompt_solids.clone()
        } else if self.mode == 0 && self.axis.is_none() {
            self.prompt_axis.clone()
        } else {
            self.prompt_points[self.points.len().min(2)].clone()
        }
    }
    fn is_selection_gathering(&self) -> bool {
        self.solids.is_empty()
    }
    fn accepts_drag_selection(&self) -> bool {
        true
    }
    fn on_selection_complete(&mut self, handles: Vec<Handle>) -> CommandStep {
        self.solids = handles;
        self.step()
    }
    fn needs_object_pick(&self) -> bool {
        !self.solids.is_empty() && self.mode == 0 && self.axis.is_none()
    }
    fn on_object_pick(&mut self, handle: Handle, _pt: [f64; 3]) -> CommandStep {
        self.axis = Some(handle);
        self.step()
    }
    fn on_point(&mut self, pt: [f64; 3]) -> CommandStep {
        if self.solids.is_empty() || self.needs_object_pick() {
            return CommandStep::NeedPoint;
        }
        self.points.push(pt);
        self.step()
    }
    fn on_cursor_move(&mut self, pt: [f64; 3]) -> Vec<PreviewWire> {
        match self.points.as_slice() {
            [] => Vec::new(),
            pts => {
                let mut p = pts.to_vec();
                p.push(pt);
                vec![PreviewWire::Polyline {
                    points: p,
                    color: Some([1.0, 0.4, 0.2, 1.0]),
                }]
            }
        }
    }
}

/// What a gathered selection is for.
#[derive(Clone, Copy)]
pub enum SelectionKind {
    /// Solids to assign to an estimation domain.
    DomainSolids,
    /// Mapped traces to project along their dip.
    Projection,
}

/// Gathers a selection (pick or window) and hands it to a follow-up command.
pub struct PickSelection {
    prompt: String,
    kind: SelectionKind,
}

impl PickSelection {
    pub fn new(prompt: String, kind: SelectionKind) -> Self {
        PickSelection { prompt, kind }
    }
}

impl InteractiveCommand for PickSelection {
    fn prompt(&self) -> String {
        self.prompt.clone()
    }
    fn on_point(&mut self, _pt: [f64; 3]) -> CommandStep {
        CommandStep::NeedPoint
    }
    fn is_selection_gathering(&self) -> bool {
        true
    }
    fn accepts_drag_selection(&self) -> bool {
        true
    }
    fn on_selection_complete(&mut self, handles: Vec<Handle>) -> CommandStep {
        if handles.is_empty() {
            return CommandStep::NeedPoint;
        }
        let (pending, next) = match self.kind {
            SelectionKind::DomainSolids => (Pending::DomainSolids(handles), "MIN_RS_SETSOLID_EXEC"),
            SelectionKind::Projection => (Pending::Projection(handles), "MIN_GM_PROJECT_EXEC"),
        };
        set_pending(pending);
        CommandStep::Dispatch(next.into())
    }
}

/// Two points defining a vertical section of the block model.
pub struct PickViewPoints {
    prompts: [String; 2],
    first: Option<Vec3>,
}

impl PickViewPoints {
    pub fn new(prompts: [String; 2]) -> Self {
        PickViewPoints { prompts, first: None }
    }
}

impl InteractiveCommand for PickViewPoints {
    fn prompt(&self) -> String {
        self.prompts[usize::from(self.first.is_some())].clone()
    }
    fn on_point(&mut self, pt: [f64; 3]) -> CommandStep {
        match self.first {
            None => {
                self.first = Some(pt);
                CommandStep::NeedPoint
            }
            Some(p) => {
                set_pending(Pending::BmView {
                    axis: None,
                    points: vec![p, pt],
                });
                CommandStep::Dispatch("MIN_RS_BMVIEW_EXEC".into())
            }
        }
    }
    fn on_cursor_move(&mut self, pt: [f64; 3]) -> Vec<PreviewWire> {
        self.first.map_or_else(Vec::new, |p| {
            vec![PreviewWire::Polyline {
                points: vec![p, pt],
                color: Some([0.3, 0.8, 1.0, 1.0]),
            }]
        })
    }
}
