//! Settings of the resource-estimation modules (saved in the drawing with
//! the rest of `ProjectConfig`).

use std::collections::{BTreeMap, HashMap};

use geostat_engine::bridge::DomainRule;
use geostat_engine::composite::{CompositeConfig, Remainder};
use geostat_engine::model::{Kind, Orientation, Structure, VariogramModel};
use geostat_engine::search::Neighborhood;
use geostat_engine::surface::SurfaceParams;
use ocs_plugin_api::dialog::DialogValue;
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
    /// Contact-surface side: 0 none, 1 above, 2 below.
    pub side: usize,
}

impl Default for DomainCfg {
    fn default() -> Self {
        DomainCfg {
            name: String::new(),
            values: String::new(),
            density: 2.7,
            cap: 0.0,
            side: 0,
        }
    }
}

impl DomainCfg {
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

    pub fn domain_names(&self) -> Vec<String> {
        self.domains.iter().map(|d| d.name.clone()).collect()
    }

    /// Saved model of a domain (falls back to the working model).
    pub fn model_of(&self, name: &str) -> Option<&ModelCfg> {
        self.vario.models.get(name)
    }

    /// Applies flattened panel values.
    pub fn apply(&mut self, v: &HashMap<String, DialogValue>) {
        let num = |k: &str| match v.get(k) {
            Some(DialogValue::Number(n)) => Some(*n),
            Some(DialogValue::Text(s)) => s.trim().replace(',', ".").parse().ok(),
            _ => None,
        };
        let text = |k: &str| match v.get(k) {
            Some(DialogValue::Text(s)) | Some(DialogValue::FilePath(s)) => Some(s.clone()),
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
        let table = |k: &str| match v.get(k) {
            Some(DialogValue::Table(r)) => Some(r.clone()),
            _ => None,
        };
        macro_rules! set {
            ($dst:expr, $src:expr) => {
                if let Some(x) = $src {
                    $dst = x;
                }
            };
        }
        set!(self.comp_length, num("rs_comp_len").filter(|x| *x > 0.0));
        set!(self.comp_remainder, choice("rs_comp_rem"));
        set!(self.comp_min_fraction, num("rs_comp_minfrac"));
        set!(self.comp_min_coverage, num("rs_comp_mincov"));
        set!(self.dc_min, num("rs_dc_min"));
        set!(self.dc_max, num("rs_dc_max"));
        set!(self.dc_steps, num("rs_dc_steps"));
        set!(self.dc_zratio, num("rs_dc_z"));
        set!(self.unit, text("rs_unit").filter(|s| !s.trim().is_empty()));
        if let Some(rows) = table("rs_domains") {
            let num_of = |r: &Vec<String>, i: usize, d: f64| {
                r.get(i)
                    .and_then(|s| s.trim().replace(',', ".").parse::<f64>().ok())
                    .unwrap_or(d)
            };
            self.domains = rows
                .iter()
                .filter(|r| r.first().is_some_and(|n| !n.trim().is_empty()))
                .map(|r| DomainCfg {
                    name: r[0].trim().to_string(),
                    values: r.get(1).cloned().unwrap_or_default(),
                    density: num_of(r, 2, 2.7),
                    cap: num_of(r, 3, 0.0),
                    side: match r.get(4).map(|s| s.trim().to_lowercase()).as_deref() {
                        Some("encima") | Some("above") | Some("acima") => 1,
                        Some("debajo") | Some("below") | Some("abaixo") => 2,
                        _ => 0,
                    },
                })
                .collect();
        }
        let c = &mut self.contact;
        set!(c.above, text("rs_ct_above"));
        set!(c.below, text("rs_ct_below"));
        set!(c.step, num("rs_ct_step").filter(|x| *x > 0.0));
        set!(c.kind, choice("rs_ct_kind"));
        set!(c.nugget, num("rs_ct_nugget"));
        set!(c.sill, num("rs_ct_sill"));
        set!(c.range, num("rs_ct_range"));
        set!(c.radius, num("rs_ct_radius"));
        set!(c.min_samples, num("rs_ct_min"));
        set!(c.max_samples, num("rs_ct_max"));
        let g = &mut self.vario;
        set!(g.domain, choice("rs_v_domain"));
        set!(g.lag, num("rs_v_lag").filter(|x| *x > 0.0));
        set!(g.nlags, num("rs_v_nlags"));
        set!(g.az_tol, num("rs_v_aztol"));
        set!(g.dip_tol, num("rs_v_diptol"));
        set!(g.bw_h, num("rs_v_bwh"));
        set!(g.bw_v, num("rs_v_bwv"));
        set!(g.omni, flag("rs_v_omni"));
        set!(g.fit_kind1, choice("rs_v_fitkind"));
        set!(g.fit_two, flag("rs_v_fittwo"));
        set!(g.fix_nugget, flag("rs_v_fixnug"));
        let w = &mut g.work;
        set!(w.azimuth, num("rs_m_az"));
        set!(w.dip, num("rs_m_dip"));
        set!(w.rake, num("rs_m_rake"));
        set!(w.nugget, num("rs_m_nugget"));
        for (i, s) in w.structures.iter_mut().enumerate() {
            let k = i + 1;
            set!(s.kind, choice(&format!("rs_m_s{k}_kind")));
            set!(s.sill, num(&format!("rs_m_s{k}_sill")));
            for (a, r) in s.ranges.iter_mut().enumerate() {
                set!(*r, num(&format!("rs_m_s{k}_r{}", a + 1)));
            }
        }
        let b = &mut self.bm;
        set!(b.auto_extent, flag("rs_b_auto"));
        for (a, name) in ["x", "y", "z"].iter().enumerate() {
            set!(b.origin[a], num(&format!("rs_b_o{name}")));
            set!(b.counts[a], num(&format!("rs_b_n{name}")));
            set!(b.size[a], num(&format!("rs_b_s{name}")).filter(|x| *x > 0.0));
            set!(b.disc[a], num(&format!("rs_b_d{name}")));
            set!(b.radii[a], num(&format!("rs_b_r{}", a + 1)).filter(|x| *x > 0.0));
        }
        set!(b.pad, num("rs_b_pad"));
        set!(b.rotation, num("rs_b_rot"));
        set!(b.ordinary, flag("rs_b_ok"));
        set!(b.simple, flag("rs_b_sk"));
        set!(b.idw, flag("rs_b_idw"));
        set!(b.idw_power, num("rs_b_idwp"));
        set!(b.nearest, flag("rs_b_nn"));
        set!(b.min_samples, num("rs_b_min"));
        set!(b.max_samples, num("rs_b_max"));
        set!(b.max_per_octant, num("rs_b_oct"));
        set!(b.max_per_hole, num("rs_b_hole"));
        set!(b.pass2, num("rs_b_p2"));
        set!(b.pass2_min, num("rs_b_p2min"));
        set!(b.pass3, num("rs_b_p3"));
        set!(b.pass3_min, num("rs_b_p3min"));
        set!(b.min_fraction, num("rs_b_minfrac"));
        set!(b.folder, text("rs_b_folder"));
        set!(b.view_mode, choice("rs_w_mode"));
        set!(b.view_attr, choice("rs_w_attr"));
        set!(b.view_cutoff, num("rs_w_cutoff"));
        set!(b.view_z_from, num("rs_w_zfrom"));
        set!(b.view_z_to, num("rs_w_zto"));
        set!(b.view_z_step, num("rs_w_zstep").filter(|x| *x > 0.0));
        set!(b.view_spacing, num("rs_w_spacing"));
        set!(b.view_count, num("rs_w_count"));
        set!(b.view_interval, num("rs_w_interval"));
        set!(b.view_domain, choice("rs_w_domain"));
        set!(b.view_limit, num("rs_w_limit"));
        set!(b.swath_axis, choice("rs_w_axis"));
        set!(b.swath_width, num("rs_w_swath"));
        set!(b.chart, choice("rs_w_chart"));
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
        let mut c = ResourcesConfig::default();
        let v: HashMap<String, DialogValue> = [
            ("rs_comp_len".to_string(), DialogValue::Number(2.0)),
            (
                "rs_comp_rem".to_string(),
                DialogValue::Choice {
                    index: 1,
                    text: String::new(),
                },
            ),
            (
                "rs_domains".to_string(),
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
            ("rs_m_s2_sill".to_string(), DialogValue::Number(4.5)),
            ("rs_b_sz".to_string(), DialogValue::Number(12.0)),
            ("rs_b_sx".to_string(), DialogValue::Number(-3.0)),
        ]
        .into_iter()
        .collect();
        c.apply(&v);
        assert_eq!((c.comp_length, c.comp_remainder), (2.0, 1));
        assert_eq!(c.domains.len(), 2);
        assert_eq!(c.domains[0].value_list(), vec!["OXIDO", "OX"]);
        assert_eq!(
            (c.domains[0].density, c.domains[0].cap, c.domains[0].side),
            (2.4, 30.0, 1)
        );
        assert_eq!((c.domains[1].density, c.domains[1].side), (2.7, 0));
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
