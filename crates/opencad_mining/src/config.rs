//! Project settings: what the panels show, persisted in the drawing (xrecord
//! `OPENMINING/PROJECT`, JSON) so a DWG reopens with the same configuration.

use std::collections::{BTreeMap, HashMap};

use drillhole_engine::color::{ColorScheme, CustomRange};
use drillhole_engine::import::{ColumnMap, Field};
use drillhole_engine::pipeline::RangeScope;
use ocs_plugin_api::dialog::DialogValue;
use serde::{Deserialize, Serialize};
use solids_engine::alignment::{CurveSpec, Element, Grade, Turn};
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
    pub ax: AxesConfig,
    pub sc: SectionsConfig,
    pub rs: crate::rsconfig::ResourcesConfig,
    pub gt: crate::gtconfig::GeotechConfig,
    pub rm: crate::rmconfig::RockMechConfig,
    pub ch: crate::chconfig::ChannelsConfig,
    pub vn: crate::veinconfig::VeinConfig,
}

impl Default for ProjectConfig {
    fn default() -> Self {
        ProjectConfig {
            version: SETTINGS_VERSION,
            dh: Default::default(),
            ug: Default::default(),
            gm: Default::default(),
            origin: Default::default(),
            ax: Default::default(),
            sc: Default::default(),
            rs: Default::default(),
            gt: Default::default(),
            rm: Default::default(),
            ch: Default::default(),
            vn: Default::default(),
        }
    }
}

/// Axis designer (panel "Ejes").
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AxesConfig {
    pub e: f64,
    pub n: f64,
    pub z: f64,
    pub bearing: String,
    pub grade: f64,
    /// 0 = %, 1 = degrees.
    pub grade_unit: usize,
    pub length: f64,
    /// 0 = radius + length, 1 = radius + angle.
    pub curve_method: usize,
    pub radius: f64,
    pub curve_length: f64,
    pub angle: f64,
    /// 0 = right (clockwise), 1 = left.
    pub turn: usize,
    pub segment: f64,
    pub update_start: bool,
}

impl Default for AxesConfig {
    fn default() -> Self {
        AxesConfig {
            e: 0.0,
            n: 0.0,
            z: 0.0,
            bearing: "0".into(),
            grade: 0.0,
            grade_unit: 0,
            length: 20.0,
            curve_method: 0,
            radius: 15.0,
            curve_length: 10.0,
            angle: 90.0,
            turn: 0,
            segment: 1.0,
            update_start: true,
        }
    }
}

impl AxesConfig {
    pub fn grade(&self) -> Grade {
        if self.grade_unit == 1 {
            Grade::Degrees(self.grade)
        } else {
            Grade::Percent(self.grade)
        }
    }
    pub fn straight(&self) -> Element {
        Element::Straight { length: self.length }
    }
    pub fn curve(&self) -> Element {
        Element::Curve {
            radius: self.radius,
            spec: if self.curve_method == 1 {
                CurveSpec::Angle(self.angle)
            } else {
                CurveSpec::Length(self.curve_length)
            },
            turn: if self.turn == 1 { Turn::Left } else { Turn::Right },
        }
    }
}

/// Cross-section generator (panel "Secciones").
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SectionsConfig {
    /// 0 = perpendicular to an axis, 1 = vertical through 2 points,
    /// 2 = horizontal levels, 3 = through 3 points.
    pub mode: usize,
    pub interval: f64,
    pub ch_from: f64,
    pub ch_to: f64,
    pub spacing: f64,
    pub count: f64,
    pub z_from: f64,
    pub z_to: f64,
    pub z_step: f64,
    pub draw_3d: bool,
    pub draw_sheets: bool,
    pub columns: f64,
    pub gap: f64,
    pub grid: f64,
    pub text_height: f64,
    pub auto_place: bool,
    pub sheet_x: f64,
    pub sheet_y: f64,
    pub replace: bool,
}

impl Default for SectionsConfig {
    fn default() -> Self {
        SectionsConfig {
            mode: 0,
            interval: 10.0,
            ch_from: 0.0,
            ch_to: 0.0,
            spacing: 5.0,
            count: 1.0,
            z_from: 0.0,
            z_to: 0.0,
            z_step: 5.0,
            draw_3d: true,
            draw_sheets: true,
            columns: 5.0,
            gap: 4.0,
            grid: 2.0,
            text_height: 0.4,
            auto_place: true,
            sheet_x: 0.0,
            sheet_y: 0.0,
            replace: true,
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
    /// Start / anchor point of the trace.
    pub e: f64,
    pub n: f64,
    pub z: f64,
    pub bearing: String,
    pub length: f64,
    pub anchor: usize,
    /// Dip 0..90 (empty = use the value stored with the trace).
    pub dip: String,
    /// 0 = right of the trace, 1 = left, 2 = fixed azimuth.
    pub dip_dir: usize,
    pub dip_az: String,
    /// 0 = target height, 1 = height difference.
    pub proj_mode: usize,
    pub proj_value: f64,
    // Metadata stored with the trace (never drawn as text).
    pub rock_type: String,
    pub thickness: String,
    pub mineralization: String,
    pub alteration: String,
    pub structure: String,
    pub grade: String,
    pub sample: String,
}

impl Default for GeoMapConfig {
    fn default() -> Self {
        GeoMapConfig {
            e: 0.0,
            n: 0.0,
            z: 0.0,
            bearing: "N45E".into(),
            length: 2.0,
            anchor: 1,
            dip: String::new(),
            dip_dir: 0,
            dip_az: String::new(),
            proj_mode: 0,
            proj_value: 0.0,
            rock_type: String::new(),
            thickness: String::new(),
            mineralization: String::new(),
            alteration: String::new(),
            structure: String::new(),
            grade: String::new(),
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

    /// Applies flattened panel values (control id -> value): the controls of
    /// the panels built from the operation definitions.
    pub fn apply(&mut self, v: &HashMap<String, DialogValue>) {
        crate::opsbind::apply(self, v);
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
                    "dh.collar".to_string(),
                    DialogValue::FilePath("c.csv".into()),
                )])),
            ),
            (
                "dh.scheme".to_string(),
                DialogValue::Choice {
                    index: 2,
                    text: "x".into(),
                },
            ),
            (
                "dh.custom_ranges".to_string(),
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
