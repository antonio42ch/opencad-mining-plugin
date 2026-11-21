//! Settings of the resource-estimation modules (saved in the drawing with
//! the rest of `ProjectConfig`).

use std::collections::BTreeMap;

use geostat_engine::bridge::DomainRule;
use geostat_engine::composite::{CompositeConfig, Remainder};
use geostat_engine::model::{Kind, Orientation, Structure, VariogramModel};
use geostat_engine::search::Neighborhood;
use geostat_engine::surface::SurfaceParams;
use serde::{Deserialize, Serialize};

pub const KINDS: [Kind; 3] = [Kind::Spherical, Kind::Exponential, Kind::Gaussian];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DomainCfg {
    pub name: String,
    /// Geology attribute values separated by `;`.
    pub values: String,
    /// Bulk density t/m3.
    pub density: f64,
    /// Top-cut of the grade (0 = none).
    pub cap: f64,
    /// Legacy (v0.4): side of the single contact surface, 0 none, 1 above,
    /// 2 below. Read for old drawings; new settings use `surface`.
    #[serde(skip_serializing)]
    pub side: usize,
    /// Surface conditions as typed: `Encima C1`, `Debajo C1; Encima C2`…
    pub surface: String,
}

impl Default for DomainCfg {
    fn default() -> Self {
        DomainCfg {
            name: String::new(),
            values: String::new(),
            density: 2.7,
            cap: 0.0,
            side: 0,
            surface: String::new(),
        }
    }
}

/// One contact surface: its name and the geology above / below it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ContactRow {
    pub name: String,
    pub above: String,
    pub below: String,
}

/// Words accepted for "above" / "below" (es, en, pt).
fn side_word(w: &str) -> Option<bool> {
    match w.trim().to_lowercase().as_str() {
        "encima" | "above" | "acima" | "sobre" => Some(true),
        "debajo" | "below" | "abaixo" | "bajo" => Some(false),
        _ => None,
    }
}

impl DomainCfg {
    /// Surface conditions `(surface name, above?)`; an empty name means the
    /// first contact surface. Unknown words are ignored.
    pub fn conditions(&self) -> Vec<(String, bool)> {
        let parsed: Vec<(String, bool)> = self
            .surface
            .split([';', ','])
            .filter_map(|part| {
                let mut words = part.split_whitespace();
                let above = side_word(words.next()?)?;
                Some((words.collect::<Vec<_>>().join(" "), above))
            })
            .collect();
        if parsed.is_empty() && self.surface.trim().is_empty() && self.side != 0 {
            return vec![(String::new(), self.side == 1)];
        }
        parsed
    }

    pub fn value_list(&self) -> Vec<String> {
        self.values
            .split(';')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct StructCfg {
    pub kind: usize,
    pub sill: f64,
    pub ranges: [f64; 3],
}

impl Default for StructCfg {
    fn default() -> Self {
        StructCfg {
            kind: 0,
            sill: 0.0,
            ranges: [100.0, 100.0, 50.0],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ModelCfg {
    pub nugget: f64,
    pub structures: [StructCfg; 2],
    pub azimuth: f64,
    pub dip: f64,
    pub rake: f64,
}

impl Default for ModelCfg {
    fn default() -> Self {
        ModelCfg {
            nugget: 1.0,
            structures: [
                StructCfg {
                    sill: 5.0,
                    ..Default::default()
                },
                StructCfg::default(),
            ],
            azimuth: 0.0,
            dip: 0.0,
            rake: 0.0,
        }
    }
}

impl ModelCfg {
    /// Structures with a positive sill (a zero sill means "unused").
    pub fn model(&self) -> VariogramModel {
        VariogramModel {
            nugget: self.nugget.max(0.0),
            structures: self
                .structures
                .iter()
                .filter(|s| s.sill > 0.0)
                .map(|s| Structure {
                    kind: KINDS[s.kind.min(2)],
                    sill: s.sill,
                    ranges: s.ranges,
                })
                .collect(),
            orientation: self.orientation(),
        }
    }

    pub fn orientation(&self) -> Orientation {
        Orientation {
            azimuth: self.azimuth,
            dip: self.dip,
            rake: self.rake,
        }
    }

    pub fn from_model(m: &VariogramModel) -> Self {
        let mut c = ModelCfg {
            nugget: m.nugget,
            structures: [StructCfg::default(); 2],
            azimuth: m.orientation.azimuth,
            dip: m.orientation.dip,
            rake: m.orientation.rake,
        };
        c.structures[0].sill = 0.0;
        for (slot, s) in c.structures.iter_mut().zip(&m.structures) {
            *slot = StructCfg {
                kind: KINDS.iter().position(|k| *k == s.kind).unwrap_or(0),
                sill: s.sill,
                ranges: s.ranges,
            };
        }
        c
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ContactCfg {
    pub above: String,
    pub below: String,
    pub step: f64,
    pub kind: usize,
    pub nugget: f64,
    pub sill: f64,
    pub range: f64,
    pub radius: f64,
    pub min_samples: f64,
    pub max_samples: f64,
}

impl Default for ContactCfg {
    fn default() -> Self {
        ContactCfg {
            above: String::new(),
            below: String::new(),
            step: 25.0,
            kind: 0,
            nugget: 0.0,
            sill: 100.0,
            range: 300.0,
            radius: 250.0,
            min_samples: 3.0,
            max_samples: 12.0,
        }
    }
}

impl ContactCfg {
    pub fn params(&self) -> SurfaceParams {
        SurfaceParams {
            kind: KINDS[self.kind.min(2)],
            nugget: self.nugget,
            sill: self.sill,
            range: self.range,
            radius: self.radius,
            min_samples: self.min_samples.round().max(1.0) as usize,
            max_samples: self.max_samples.round().max(1.0) as usize,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct VarioCfg {
    /// Domain being worked on in the variography panel.
    pub domain: usize,
    /// Working model (the panel fields).
    pub work: ModelCfg,
    pub lag: f64,
    pub nlags: f64,
    pub az_tol: f64,
    pub dip_tol: f64,
    pub bw_h: f64,
    pub bw_v: f64,
    pub omni: bool,
    pub fit_kind1: usize,
    pub fit_two: bool,
    pub fix_nugget: bool,
    /// Saved model per domain name.
    pub models: BTreeMap<String, ModelCfg>,
}

impl Default for VarioCfg {
    fn default() -> Self {
        VarioCfg {
            domain: 0,
            work: ModelCfg::default(),
            lag: 50.0,
            nlags: 8.0,
            az_tol: 22.5,
            dip_tol: 22.5,
            bw_h: 0.0,
            bw_v: 0.0,
            omni: true,
            fit_kind1: 0,
            fit_two: false,
            fix_nugget: false,
            models: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct BlockCfg {
    pub auto_extent: bool,
    pub origin: [f64; 3],
    /// Extent in blocks when not automatic.
    pub counts: [f64; 3],
    pub pad: f64,
    pub size: [f64; 3],
    pub rotation: f64,
    pub disc: [f64; 3],
    pub ordinary: bool,
    pub simple: bool,
    pub idw: bool,
    pub idw_power: f64,
    pub nearest: bool,
    pub radii: [f64; 3],
    pub min_samples: f64,
    pub max_samples: f64,
    pub max_per_octant: f64,
    pub max_per_hole: f64,
    /// Radius multipliers of passes 2 and 3 (0 = pass off) and their minimum samples.
    pub pass2: f64,
    pub pass2_min: f64,
    pub pass3: f64,
    pub pass3_min: f64,
    pub min_fraction: f64,
    pub folder: String,
    // View
    pub view_mode: usize,
    pub view_attr: usize,
    pub view_cutoff: f64,
    pub view_z_from: f64,
    pub view_z_to: f64,
    pub view_z_step: f64,
    pub view_spacing: f64,
    pub view_count: f64,
    pub view_interval: f64,
    pub view_domain: usize,
    pub view_limit: f64,
    /// Geotechnical views only: 0 show all, 1 only values below `view_threshold`, 2 only above.
    pub view_filter: usize,
    pub view_threshold: f64,
    /// Geotechnical views only: keep the blocks on one side of a plane (point, dip, dip
    /// direction; side 0 = the side the normal points to, 1 = the other).
    pub view_cut: bool,
    pub view_cut_point: [f64; 3],
    pub view_cut_dip: f64,
    pub view_cut_dipdir: f64,
    pub view_cut_side: usize,
    pub swath_axis: usize,
    pub swath_width: f64,
    pub chart: usize,
}

impl Default for BlockCfg {
    fn default() -> Self {
        BlockCfg {
            auto_extent: true,
            origin: [0.0; 3],
            counts: [50.0, 50.0, 30.0],
            pad: 0.0,
            size: [10.0, 10.0, 12.0],
            rotation: 0.0,
            disc: [4.0, 4.0, 4.0],
            ordinary: true,
            simple: false,
            idw: false,
            idw_power: 2.0,
            nearest: true,
            radii: [200.0, 150.0, 50.0],
            min_samples: 4.0,
            max_samples: 8.0,
            max_per_octant: 0.0,
            max_per_hole: 0.0,
            pass2: 2.0,
            pass2_min: 3.0,
            pass3: 0.0,
            pass3_min: 2.0,
            min_fraction: 0.01,
            folder: String::new(),
            view_mode: 0,
            view_attr: 0,
            view_cutoff: 0.0,
            view_z_from: 0.0,
            view_z_to: 0.0,
            view_z_step: 12.0,
            view_spacing: 20.0,
            view_count: 1.0,
            view_interval: 20.0,
            view_domain: 0,
            view_limit: 20_000.0,
            view_filter: 0,
            view_threshold: 3.0,
            view_cut: false,
            view_cut_point: [0.0; 3],
            view_cut_dip: 90.0,
            view_cut_dipdir: 0.0,
            view_cut_side: 0,
            swath_axis: 2,
            swath_width: 24.0,
            chart: 0,
        }
    }
}

impl BlockCfg {
    pub fn discretization(&self) -> [usize; 3] {
        self.disc.map(|v| v.round().max(1.0) as usize)
    }

    /// Neighbourhood passes for a domain whose model has `orientation`.
    pub fn passes(&self, orientation: Orientation) -> Vec<Neighborhood> {
        let base = Neighborhood {
            radii: self.radii,
            orientation,
            min_samples: self.min_samples.round().max(1.0) as usize,
            max_samples: self.max_samples.round().max(1.0) as usize,
            max_per_octant: self.max_per_octant.round().max(0.0) as usize,
            max_per_hole: self.max_per_hole.round().max(0.0) as usize,
        };
        let mut v = vec![base.clone()];
        for (mult, min) in [(self.pass2, self.pass2_min), (self.pass3, self.pass3_min)] {
            if mult > 1.0 {
                v.push(Neighborhood {
                    min_samples: min.round().max(1.0) as usize,
                    ..base.scaled(mult)
                });
            }
        }
        v
    }
}

/// Attribute and domain list the cached composites were made with.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct CompositeStamp {
    pub variable: String,
    pub domains: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ResourcesConfig {
    pub comp_length: f64,
    /// 0 merge, 1 discard, 2 keep.
    pub comp_remainder: usize,
    pub comp_min_fraction: f64,
    pub comp_min_coverage: f64,
    pub domains: Vec<DomainCfg>,
    pub dc_min: f64,
    pub dc_max: f64,
    pub dc_steps: f64,
    pub dc_zratio: f64,
    pub unit: String,
    pub contact: ContactCfg,
    pub vario: VarioCfg,
    pub bm: BlockCfg,
    pub composited: CompositeStamp,
    /// Contact surfaces (name, geology above, geology below).
    pub contacts: Vec<ContactRow>,
    /// Domain that `rs_setsolid` assigns solids to (index).
    pub target: usize,
}

impl Default for ResourcesConfig {
    fn default() -> Self {
        ResourcesConfig {
            comp_length: 4.0,
            comp_remainder: 0,
            comp_min_fraction: 0.5,
            comp_min_coverage: 0.5,
            domains: Vec::new(),
            dc_min: 20.0,
            dc_max: 200.0,
            dc_steps: 8.0,
            dc_zratio: 0.5,
            unit: "ppm".into(),
            contact: ContactCfg::default(),
            vario: VarioCfg::default(),
            bm: BlockCfg::default(),
            composited: CompositeStamp::default(),
            contacts: Vec::new(),
            target: 0,
        }
    }
}

impl ResourcesConfig {
    pub fn composite_config(&self) -> CompositeConfig {
        CompositeConfig {
            length: self.comp_length,
            min_fraction: self.comp_min_fraction,
            remainder: [Remainder::Merge, Remainder::Discard, Remainder::Keep][self.comp_remainder.min(2)],
            min_coverage: self.comp_min_coverage,
        }
    }

    pub fn rules(&self) -> Vec<DomainRule> {
        self.domains
            .iter()
            .map(|d| DomainRule {
                name: d.name.clone(),
                values: d.value_list(),
            })
            .collect()
    }

    pub fn decluster_sizes(&self) -> Vec<f64> {
        let n = self.dc_steps.round().max(1.0) as usize;
        if n == 1 || self.dc_max <= self.dc_min {
            return vec![self.dc_min.max(1.0)];
        }
        (0..n)
            .map(|i| self.dc_min + (self.dc_max - self.dc_min) * i as f64 / (n - 1) as f64)
            .collect()
    }

    /// Contact surfaces in use: the table, or the single v0.4 contact.
    pub fn contact_rows(&self) -> Vec<ContactRow> {
        if !self.contacts.is_empty() {
            return self.contacts.clone();
        }
        let c = &self.contact;
        if c.above.trim().is_empty() && c.below.trim().is_empty() {
            Vec::new()
        } else {
            vec![ContactRow {
                name: "C1".into(),
                above: c.above.clone(),
                below: c.below.clone(),
            }]
        }
    }

    /// v0.4 domains kept the side of the single contact in `side` (not
    /// serialized any more): write it as the equivalent surface word so it
    /// survives a round trip through the settings JSON.
    pub fn migrate_legacy(&mut self) {
        for d in &mut self.domains {
            if d.surface.trim().is_empty() && d.side != 0 {
                d.surface = if d.side == 1 { "above" } else { "below" }.into();
                d.side = 0;
            }
        }
    }

    pub fn domain_names(&self) -> Vec<String> {
        self.domains.iter().map(|d| d.name.clone()).collect()
    }

    /// Saved model of a domain (falls back to the working model).
    pub fn model_of(&self, name: &str) -> Option<&ModelCfg> {
        self.vario.models.get(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_roundtrip_through_the_engine_type() {
        let mut c = ModelCfg {
            nugget: 2.0,
            azimuth: 325.0,
            ..Default::default()
        };
        c.structures[0] = StructCfg {
            kind: 1,
            sill: 7.0,
            ranges: [120.0, 80.0, 30.0],
        };
        c.structures[1] = StructCfg {
            kind: 2,
            sill: 3.0,
            ranges: [300.0, 200.0, 60.0],
        };
        let m = c.model();
        assert_eq!(m.structures.len(), 2);
        assert_eq!(m.structures[0].kind, Kind::Exponential);
        assert_eq!(ModelCfg::from_model(&m), c);
        // A zero sill means "unused structure".
        c.structures[1].sill = 0.0;
        assert_eq!(c.model().structures.len(), 1);
    }

    #[test]
    fn surface_conditions_parse_and_legacy_settings_still_work() {
        let d = |s: &str| DomainCfg {
            surface: s.into(),
            ..Default::default()
        };
        assert_eq!(d("Encima C1").conditions(), vec![("C1".to_string(), true)]);
        assert_eq!(
            d("debajo C1; Encima C2").conditions(),
            vec![("C1".to_string(), false), ("C2".to_string(), true)]
        );
        assert_eq!(d("Below").conditions(), vec![(String::new(), false)]);
        assert_eq!(d("Acima Contato 2").conditions(), vec![("Contato 2".to_string(), true)]);
        assert!(d("").conditions().is_empty() && d("algo").conditions().is_empty());
        // v0.4 drawings stored a numeric side for the single surface.
        let legacy: DomainCfg = serde_json::from_str(r#"{"name":"Ox","side":1}"#).unwrap();
        assert_eq!(legacy.conditions(), vec![(String::new(), true)]);
        assert!(
            !serde_json::to_string(&legacy).unwrap().contains("side"),
            "new files do not write the legacy field"
        );
        let mut rc = ResourcesConfig::default();
        assert!(rc.contact_rows().is_empty());
        rc.contact.above = "OXIDO".into();
        rc.contact.below = "SULFURO".into();
        assert_eq!(rc.contact_rows()[0].name, "C1");
        rc.contacts = vec![ContactRow {
            name: "A".into(),
            above: "X".into(),
            below: "Y".into(),
        }];
        assert_eq!(rc.contact_rows()[0].name, "A");
    }

    #[test]
    fn passes_follow_the_configuration() {
        let b = BlockCfg {
            pass2: 2.0,
            pass3: 3.0,
            pass2_min: 3.0,
            pass3_min: 1.0,
            ..Default::default()
        };
        let p = b.passes(Orientation {
            azimuth: 56.0,
            dip: 0.0,
            rake: 0.0,
        });
        assert_eq!(p.len(), 3);
        assert_eq!(p[0].radii, [200.0, 150.0, 50.0]);
        assert_eq!((p[1].radii[0], p[1].min_samples), (400.0, 3));
        assert_eq!((p[2].radii[2], p[2].min_samples), (150.0, 1));
        assert!(p.iter().all(|n| n.orientation.azimuth == 56.0));
        assert_eq!(
            BlockCfg {
                pass2: 0.0,
                ..Default::default()
            }
            .passes(Orientation::default())
            .len(),
            1
        );
    }

    #[test]
    fn json_is_forward_compatible_and_panel_values_apply() {
        use ocs_plugin_api::dialog::DialogValue;
        use std::collections::HashMap;
        let mut pc = crate::config::ProjectConfig::default();
        let v: HashMap<String, DialogValue> = [
            ("rs.comp_length".to_string(), DialogValue::Number(2.0)),
            (
                "rs.comp_remainder".to_string(),
                DialogValue::Choice {
                    index: 1,
                    text: String::new(),
                },
            ),
            (
                "rs.domains".to_string(),
                DialogValue::Table(vec![
                    vec![
                        "Óxidos".into(),
                        "OXIDO; OX".into(),
                        "2,4".into(),
                        "30".into(),
                        "Encima".into(),
                    ],
                    vec!["".into(), "x".into()],
                    vec!["Sulfuros".into(), "SULFURO".into()],
                ]),
            ),
            ("rs.vario.work.structures.1.sill".to_string(), DialogValue::Number(4.5)),
            ("rs.bm.size.2".to_string(), DialogValue::Number(12.0)),
            ("rs.bm.size.0".to_string(), DialogValue::Number(-3.0)),
        ]
        .into_iter()
        .collect();
        pc.apply(&v);
        let c = pc.rs.clone();
        assert_eq!((c.comp_length, c.comp_remainder), (2.0, 1));
        assert_eq!(c.domains.len(), 2);
        assert_eq!(c.domains[0].value_list(), vec!["OXIDO", "OX"]);
        assert_eq!(
            (c.domains[0].density, c.domains[0].cap, c.domains[0].conditions().len()),
            (2.4, 30.0, 1)
        );
        assert_eq!((c.domains[1].density, c.domains[1].conditions().len()), (2.7, 0));
        assert_eq!(c.domains[0].conditions(), vec![(String::new(), true)]);
        assert_eq!(c.vario.work.structures[1].sill, 4.5);
        assert_eq!(c.bm.size, [10.0, 10.0, 12.0], "non-positive size is ignored");
        let back: ResourcesConfig = serde_json::from_str(&serde_json::to_string(&c).unwrap()).unwrap();
        assert_eq!(back, c);
        let partial: ResourcesConfig = serde_json::from_str(r#"{"comp_length":3.0,"future":1}"#).unwrap();
        assert_eq!(partial.comp_length, 3.0);
        assert_eq!(partial.bm, BlockCfg::default());
        assert_eq!(c.decluster_sizes().len(), 8);
    }
}
