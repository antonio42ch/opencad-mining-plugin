//! Interactive picks. They carry no `HostApi`, so when the input is complete
//! they park it in `state::Pending` and dispatch a follow-up command that
//! does the work with full host access.

use ocs_plugin_api::host::{CommandStep, Handle, InteractiveCommand, PreviewWire};

use crate::config::GeoMapConfig;
use crate::state::{set_pending, Pending};

pub enum PickKind {
    Axis { custom_profile: bool },
    Boundaries { vertical: bool },
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
        set_pending(match self.kind {
            PickKind::Axis { custom_profile } => Pending::Axis {
                axis: p[0],
                profile: custom_profile.then(|| p[1]),
            },
            PickKind::Boundaries { vertical } => Pending::Boundaries {
                vertical,
                handles: [p[0], p[1], p[2], p[3]],
            },
        });
        CommandStep::Dispatch("MIN_UG_RUN".into())
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
