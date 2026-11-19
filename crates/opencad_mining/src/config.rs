//! Project settings: what the panels show, persisted in the drawing (xrecord
//! `OPENMINING/PROJECT`, JSON) so a DWG reopens with the same configuration.

use std::collections::{BTreeMap, HashMap};

use drillhole_engine::color::{ColorScheme, CustomRange};
use drillhole_engine::import::{ColumnMap, Field};
use drillhole_engine::pipeline::RangeScope;
use ocs_plugin_api::dialog::DialogValue;
use serde::{Deserialize, Serialize};
use solids_engine::section::{HAlign, Profile, VAlign};

pub const SETTINGS_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ProjectConfig {
    pub version: u32,
    pub dh: DrillholeConfig,
    pub ug: WorkingsConfig,
    pub gm: GeoMapConfig,
    pub origin: OriginConfig,
}

impl Default for ProjectConfig {
    fn default() -> Self {
        ProjectConfig {
            version: SETTINGS_VERSION,
            dh: Default::default(),
            ug: Default::default(),
            gm: Default::default(),
            origin: Default::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DrillholeConfig {
    pub collar: String,
    pub survey: String,
    pub assays: String,
    pub geo: String,
    /// table key -> field key -> header text.
    pub column_maps: BTreeMap<String, BTreeMap<String, String>>,
    pub draw_trace: bool,
    pub draw_collar: bool,
    pub draw_assays: bool,
    pub draw_geo: bool,
    pub draw_labels: bool,
    pub assay_attr: String,
    pub geo_attr: String,
    pub scheme: usize,
    pub scope: usize,
    pub label_height: f64,
    /// (min, max, colour name)
    pub custom_ranges: Vec<(f64, f64, String)>,
    /// value -> colour name
    pub geo_colors: BTreeMap<String, String>,
}

impl Default for DrillholeConfig {
    fn default() -> Self {
        DrillholeConfig {
            collar: String::new(),
            survey: String::new(),
            assays: String::new(),
            geo: String::new(),
            column_maps: BTreeMap::new(),
            draw_trace: true,
            draw_collar: true,
            draw_assays: true,
            draw_geo: false,
            draw_labels: false,
            assay_attr: String::new(),
            geo_attr: String::new(),
            scheme: 0,
            scope: 0,
            label_height: 2.5,
            custom_ranges: Vec::new(),
            geo_colors: BTreeMap::new(),
        }
    }
}

impl DrillholeConfig {
    pub fn scheme(&self) -> ColorScheme {
        [
            ColorScheme::BlueToRed,
            ColorScheme::GreenToRed,
            ColorScheme::Rainbow,
            ColorScheme::Custom,
        ][self.scheme.min(3)]
    }
    pub fn scope(&self) -> RangeScope {
        if self.scope == 1 {
            RangeScope::PerHole
        } else {
            RangeScope::Global
        }
    }
    pub fn custom_ranges(&self) -> Vec<CustomRange> {
        self.custom_ranges
            .iter()
            .map(|(min, max, c)| CustomRange {
                min: *min,
                max: *max,
                aci: drillhole_engine::color::color_name_to_aci(c),
            })
            .collect()
    }
    pub fn custom_ranges_valid(&self) -> bool {
        !self.custom_ranges.is_empty() && self.custom_ranges.iter().all(|(a, b, c)| a < b && !c.trim().is_empty())
    }
    pub fn column_map(&self, table: &str) -> ColumnMap {
        self.column_maps
            .get(table)
            .map(|m| {
                m.iter()
                    .filter_map(|(f, h)| Field::from_key(f).map(|f| (f, h.clone())))
                    .collect()
            })
            .unwrap_or_default()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct WorkingsConfig {
    pub profile: usize,
    pub width: f64,
    pub height: f64,
    pub top_width: f64,
    pub radius: f64,
    pub wall_height: f64,
    pub interval: f64,
    pub h_align: usize,
    pub v_align: usize,
    pub draw_sections: bool,
    pub tangent_3d: bool,
}

impl Default for WorkingsConfig {
    fn default() -> Self {
        WorkingsConfig {
            profile: 0,
            width: 4.0,
            height: 4.0,
            top_width: 3.0,
            radius: 2.0,
            wall_height: 3.0,
            interval: 2.0,
            h_align: 1,
            v_align: 1,
            draw_sections: false,
            tangent_3d: false,
        }
    }
}

impl WorkingsConfig {
    /// `None` for the custom profile (it comes from a picked polyline).
    pub fn profile(&self) -> Option<Profile> {
        Some(match self.profile {
            0 => Profile::Rectangular {
                width: self.width,
                height: self.height,
            },
            1 => Profile::Trapezoidal {
                bottom: self.width,
                top: self.top_width,
                height: self.height,
            },
            2 => Profile::Circular {
                radius: self.radius,
                segments: 32,
            },
            3 => Profile::Horseshoe {
                width: self.width,
                wall_height: self.wall_height,
                arc_radius: None,
                arc_points: 17,
            },
            _ => return None,
        })
    }
    pub fn align(&self) -> (HAlign, VAlign) {
        (
            [HAlign::Left, HAlign::Center, HAlign::Right][self.h_align.min(2)],
            [VAlign::Top, VAlign::Center, VAlign::Bottom][self.v_align.min(2)],
        )
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct GeoMapConfig {
    pub bearing: String,
    pub length: f64,
    pub anchor: usize,
    pub annotate: bool,
    pub rock_type: String,
    pub thickness: String,
    pub mineralization: String,
    pub alteration: String,
    pub structure: String,
    pub grade: String,
    pub dip: String,
    pub sample: String,
}

impl Default for GeoMapConfig {
    fn default() -> Self {
        GeoMapConfig {
            bearing: "N45E".into(),
            length: 2.0,
            anchor: 1,
            annotate: true,
            rock_type: String::new(),
            thickness: String::new(),
            mineralization: String::new(),
            alteration: String::new(),
            structure: String::new(),
            grade: String::new(),
            dip: String::new(),
            sample: String::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct OriginConfig {
    pub enabled: bool,
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl ProjectConfig {
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
    pub fn from_json(s: &str) -> Option<Self> {
        serde_json::from_str(s).ok()
    }

    /// Applies flattened panel values (control id -> value).
    pub fn apply(&mut self, v: &HashMap<String, DialogValue>) {
        let text = |k: &str| match v.get(k) {
            Some(DialogValue::Text(s)) | Some(DialogValue::FilePath(s)) => Some(s.clone()),
            Some(DialogValue::Number(n)) => Some(n.to_string()),
            _ => None,
        };
        let num = |k: &str| match v.get(k) {
            Some(DialogValue::Number(n)) => Some(*n),
            Some(DialogValue::Text(s)) => s.trim().replace(',', ".").parse().ok(),
            _ => None,
        };
        let flag = |k: &str| match v.get(k) {
            Some(DialogValue::Bool(b)) => Some(*b),
            _ => None,
        };
        let choice = |k: &str| match v.get(k) {
            Some(DialogValue::Choice { index, .. }) => Some(*index),
            _ => None,
        };
        let choice_text = |k: &str| match v.get(k) {
            Some(DialogValue::Choice { text, .. }) => Some(text.clone()),
            _ => None,
        };
        let table = |k: &str| match v.get(k) {
            Some(DialogValue::Table(rows)) => Some(rows.clone()),
            _ => None,
        };
        macro_rules! set {
            ($dst:expr, $src:expr) => {
                if let Some(x) = $src {
                    $dst = x;
                }
            };
        }
        let d = &mut self.dh;
        set!(d.collar, text("dh_collar"));
        set!(d.survey, text("dh_survey"));
        set!(d.assays, text("dh_assays"));
        set!(d.geo, text("dh_geo"));
        set!(d.draw_trace, flag("dh_draw_trace"));
        set!(d.draw_collar, flag("dh_draw_collar"));
        set!(d.draw_assays, flag("dh_draw_assays"));
        set!(d.draw_geo, flag("dh_draw_geo"));
        set!(d.draw_labels, flag("dh_draw_labels"));
        set!(d.assay_attr, choice_text("dh_assay_attr").filter(|s| !s.is_empty()));
        set!(d.geo_attr, choice_text("dh_geo_attr").filter(|s| !s.is_empty()));
        set!(d.scheme, choice("dh_scheme"));
        set!(d.scope, choice("dh_scope"));
        set!(d.label_height, num("dh_label_h").filter(|h| *h > 0.0));
        if let Some(rows) = table("dh_ranges") {
            d.custom_ranges = rows
                .iter()
                .filter_map(|r| {
                    let p = |i: usize| r.get(i).and_then(|s| s.trim().replace(',', ".").parse::<f64>().ok());
                    Some((p(0)?, p(1)?, r.get(2)?.trim().to_string()))
                })
                .collect();
        }
        if let Some(rows) = table("dh_geo_colors") {
            d.geo_colors = rows
                .iter()
                .filter_map(|r| Some((r.first()?.trim().to_string(), r.get(1)?.trim().to_string())))
                .filter(|(k, _)| !k.is_empty())
                .collect();
        }
        let o = &mut self.origin;
        set!(o.enabled, flag("or_enabled"));
        set!(o.x, num("or_x"));
        set!(o.y, num("or_y"));
        set!(o.z, num("or_z"));
        let u = &mut self.ug;
        set!(u.profile, choice("ug_profile"));
        set!(u.width, num("ug_width"));
        set!(u.height, num("ug_height"));
        set!(u.top_width, num("ug_top"));
        set!(u.radius, num("ug_radius"));
        set!(u.wall_height, num("ug_wall"));
        set!(u.interval, num("ug_interval"));
        set!(u.h_align, choice("ug_halign"));
        set!(u.v_align, choice("ug_valign"));
        set!(u.draw_sections, flag("ug_sections"));
        set!(u.tangent_3d, flag("ug_tangent3d"));
        let g = &mut self.gm;
        set!(g.bearing, text("gm_bearing"));
        set!(g.length, num("gm_length"));
        set!(g.anchor, choice("gm_anchor"));
        set!(g.annotate, flag("gm_annotate"));
        set!(g.rock_type, text("gm_rock"));
        set!(g.thickness, text("gm_thickness"));
        set!(g.mineralization, text("gm_min"));
        set!(g.alteration, text("gm_alt"));
        set!(g.structure, text("gm_struct"));
        set!(g.grade, text("gm_grade"));
        set!(g.dip, text("gm_dip"));
        set!(g.sample, text("gm_sample"));
    }
}

/// Flattens nested `Fields` maps from a panel snapshot.
pub fn flatten(values: &HashMap<String, DialogValue>) -> HashMap<String, DialogValue> {
    let mut out = HashMap::new();
    fn walk(m: &HashMap<String, DialogValue>, out: &mut HashMap<String, DialogValue>) {
        for (k, v) in m {
            match v {
                DialogValue::Fields(inner) => walk(inner, out),
                other => {
                    out.insert(k.clone(), other.clone());
                }
            }
        }
    }
    walk(values, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_roundtrip_and_forward_compat() {
        let mut c = ProjectConfig::default();
        c.dh.collar = "/data/collar.csv".into();
        c.dh.column_maps
            .insert("collar".into(), BTreeMap::from([("x".into(), "ESTE_UTM".into())]));
        let back = ProjectConfig::from_json(&c.to_json()).unwrap();
        assert_eq!(back, c);
        assert_eq!(
            back.dh.column_map("collar").get(&Field::X).map(String::as_str),
            Some("ESTE_UTM")
        );
        // Unknown keys / missing sections are tolerated.
        let partial = ProjectConfig::from_json(r#"{"dh":{"collar":"a.csv"},"future":1}"#).unwrap();
        assert_eq!(partial.dh.collar, "a.csv");
        assert_eq!(partial.ug, WorkingsConfig::default());
    }

    #[test]
    fn apply_nested_panel_values() {
        let nested = HashMap::from([
            (
                "files".to_string(),
                DialogValue::Fields(HashMap::from([(
                    "dh_collar".to_string(),
                    DialogValue::FilePath("c.csv".into()),
                )])),
            ),
            (
                "dh_scheme".to_string(),
                DialogValue::Choice {
                    index: 2,
                    text: "x".into(),
                },
            ),
            (
                "dh_ranges".to_string(),
                DialogValue::Table(vec![vec!["0".into(), "1,5".into(), "Verde".into()], vec!["bad".into()]]),
            ),
        ]);
        let mut c = ProjectConfig::default();
        c.apply(&flatten(&nested));
        assert_eq!(c.dh.collar, "c.csv");
        assert_eq!(c.dh.scheme, 2);
        assert_eq!(c.dh.custom_ranges, vec![(0.0, 1.5, "Verde".to_string())]);
    }
}
