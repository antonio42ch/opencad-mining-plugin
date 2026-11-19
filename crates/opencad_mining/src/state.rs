//! Session state. Per document tab: settings + loaded data. Global: the last
//! panel values seen in notifications (fallback when the host cannot answer
//! `panel_snapshot_for_plugin`) and the pending action of an interactive pick.

use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard};

use drillhole_engine::DataSet;
use mining_core::Vec3;
use ocs_plugin_api::dialog::DialogValue;
use ocs_plugin_api::host::Handle;

use crate::config::ProjectConfig;

#[derive(Default)]
pub struct TabState {
    pub config: ProjectConfig,
    pub data: Option<DataSet>,
    /// Settings were looked up in the drawing already.
    pub settings_checked: bool,
}

/// What an interactive pick hands over to the follow-up command.
#[derive(Debug, Clone, PartialEq)]
pub enum Pending {
    Axis { axis: Handle, profile: Option<Handle> },
    Boundaries { vertical: bool, handles: [Handle; 4] },
    TracePoint(Vec3),
}

static TABS: Mutex<Option<HashMap<u64, TabState>>> = Mutex::new(None);
static PANEL_VALUES: Mutex<Option<HashMap<String, DialogValue>>> = Mutex::new(None);
static PENDING: Mutex<Option<Pending>> = Mutex::new(None);

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

pub fn with_tab<R>(tab: u64, f: impl FnOnce(&mut TabState) -> R) -> R {
    let mut g = lock(&TABS);
    f(g.get_or_insert_with(HashMap::new).entry(tab).or_default())
}

pub fn close_tab(tab: u64) {
    if let Some(m) = lock(&TABS).as_mut() {
        m.remove(&tab);
    }
}

pub fn track_value(control: String, value: DialogValue) {
    lock(&PANEL_VALUES)
        .get_or_insert_with(HashMap::new)
        .insert(control, value);
}

pub fn tracked_values() -> HashMap<String, DialogValue> {
    lock(&PANEL_VALUES).clone().unwrap_or_default()
}

pub fn set_pending(p: Pending) {
    *lock(&PENDING) = Some(p);
}

pub fn take_pending() -> Option<Pending> {
    lock(&PENDING).take()
}
