//! Validation and reporting: statistics, swath plots, grade-tonnage curves
//! of the model and of the declustered composites, resource summary.

use geostat_engine::stats::{grade_tonnage, summarize, GradeTonnage, Summary};
use geostat_engine::Sample;

use crate::model::BlockModel;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    X,
    Y,
    Z,
}

impl Axis {
    fn pick(self, p: [f64; 3]) -> f64 {
        p[self as usize]
    }
}

/// Which blocks take part: a domain (`0` = all assigned) and a minimum fraction.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Filter {
    pub domain: u8,
    pub min_fraction: f32,
}

impl Filter {
    pub fn all() -> Self {
        Filter {
            domain: 0,
            min_fraction: 0.0,
        }
    }
    fn accepts(&self, m: &BlockModel, i: usize) -> bool {
        m.domain[i] != 0
            && (self.domain == 0 || m.domain[i] == self.domain)
            && m.fraction[i] > self.min_fraction.max(0.0)
    }
}

/// Weights (tonnes) and values of the accepted blocks with a finite grade.
pub fn block_values(m: &BlockModel, column: &str, f: &Filter) -> (Vec<f64>, Vec<f64>) {
    let (Some(col), Some(ton)) = (m.column(column), m.column("tonnage")) else {
        return (Vec::new(), Vec::new());
    };
    let mut v = Vec::new();
    let mut w = Vec::new();
    for i in 0..m.len() {
        if f.accepts(m, i) && col[i].is_finite() && ton[i] > 0.0 {
            v.push(col[i] as f64);
            w.push(ton[i] as f64);
        }
    }
    (v, w)
}

pub fn block_summary(m: &BlockModel, column: &str, f: &Filter) -> Summary {
    let (v, w) = block_values(m, column, f);
    summarize(&v, Some(&w))
}

/// Grade-tonnage of the block model (tonnes above each cut-off).
pub fn model_grade_tonnage(m: &BlockModel, column: &str, f: &Filter, cutoffs: &[f64]) -> Vec<GradeTonnage> {
    let (v, w) = block_values(m, column, f);
    grade_tonnage(&v, &w, cutoffs)
}

/// Same curve for composites, weights normalised to **fractions** of the
/// total (length x decluster weight), so it compares with the model's
/// fraction of tonnage above each cut-off.
pub fn sample_grade_tonnage(samples: &[Sample], weights: &[f64], cutoffs: &[f64]) -> Vec<GradeTonnage> {
    let w: Vec<f64> = samples
        .iter()
        .zip(weights)
        .map(|(s, d)| s.length.max(1e-9) * d)
        .collect();
    let total: f64 = w.iter().sum();
    let norm: Vec<f64> = w.iter().map(|x| x / total.max(1e-300)).collect();
    let v: Vec<f64> = samples.iter().map(|s| s.value).collect();
    grade_tonnage(&v, &norm, cutoffs)
}

#[derive(Debug, Clone, PartialEq)]
pub struct SwathBin {
    pub center: f64,
    pub blocks: usize,
    /// Mean of each requested column (tonnage weighted), NaN if none.
    pub means: Vec<f64>,
}

/// Swath plot: mean of `columns` per slice of `width` along `axis` (world
/// coordinates of the block centres).
pub fn swath(m: &BlockModel, columns: &[&str], axis: Axis, width: f64, f: &Filter) -> Vec<SwathBin> {
    let width = if width > 0.0 { width } else { 1.0 };
    let cols: Vec<Option<&[f32]>> = columns.iter().map(|c| m.column(c)).collect();
    let ton = m.column("tonnage");
    let mut bins: std::collections::BTreeMap<i64, (usize, Vec<(f64, f64)>)> = std::collections::BTreeMap::new();
    for i in 0..m.len() {
        if !f.accepts(m, i) {
            continue;
        }
        let c = axis.pick(m.geometry.center(i));
        let key = (c / width).floor() as i64;
        let e = bins.entry(key).or_insert_with(|| (0, vec![(0.0, 0.0); columns.len()]));
        e.0 += 1;
        let w = ton.map_or(1.0, |t| t[i] as f64).max(1e-9);
        for (k, col) in cols.iter().enumerate() {
            if let Some(col) = col {
                if col[i].is_finite() {
                    e.1[k].0 += col[i] as f64 * w;
                    e.1[k].1 += w;
                }
            }
        }
    }
    bins.into_iter()
        .map(|(k, (n, acc))| SwathBin {
            center: (k as f64 + 0.5) * width,
            blocks: n,
            means: acc
                .into_iter()
                .map(|(s, w)| if w > 0.0 { s / w } else { f64::NAN })
                .collect(),
        })
        .collect()
}

/// Declustered mean of the composites per slice (same slicing as [`swath`]).
pub fn sample_swath(samples: &[Sample], weights: &[f64], axis: Axis, width: f64, domain: u32) -> Vec<(f64, f64)> {
    let width = if width > 0.0 { width } else { 1.0 };
    let mut bins: std::collections::BTreeMap<i64, (f64, f64)> = std::collections::BTreeMap::new();
    for (s, d) in samples.iter().zip(weights) {
        if domain != 0 && s.domain != domain {
            continue;
        }
        let w = s.length.max(1e-9) * d;
        let e = bins.entry((axis.pick(s.xyz) / width).floor() as i64).or_default();
        e.0 += s.value * w;
        e.1 += w;
    }
    bins.into_iter()
        .map(|(k, (s, w))| ((k as f64 + 0.5) * width, s / w))
        .collect()
}

/// Totals of one domain at a cut-off.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ResourceRow {
    pub domain: String,
    pub blocks: usize,
    pub tonnes: f64,
    pub grade: f64,
    /// Metal in grade units x tonnes (divide by the unit factor outside).
    pub metal: f64,
    pub volume: f64,
}

/// Resource summary per domain above `cutoff` on `column` (+ a "Total" row).
pub fn summary_by_domain(m: &BlockModel, column: &str, cutoff: f64) -> Vec<ResourceRow> {
    let (Some(col), Some(ton)) = (m.column(column), m.column("tonnage")) else {
        return Vec::new();
    };
    let mut rows: Vec<ResourceRow> = (1..=m.domain_names.len())
        .map(|d| ResourceRow {
            domain: m.domain_name(d as u8).to_string(),
            ..Default::default()
        })
        .collect();
    for i in 0..m.len() {
        let d = m.domain[i];
        if d == 0 || !col[i].is_finite() || (col[i] as f64) < cutoff {
            continue;
        }
        // Ids without a name (inconsistent model) are skipped, never indexed.
        let Some(r) = rows.get_mut(d as usize - 1) else {
            continue;
        };
        r.blocks += 1;
        r.tonnes += ton[i] as f64;
        r.metal += ton[i] as f64 * col[i] as f64;
        r.volume += m.inside_volume(i);
    }
    let mut total = ResourceRow {
        domain: "Total".into(),
        ..Default::default()
    };
    for r in &mut rows {
        r.grade = if r.tonnes > 0.0 { r.metal / r.tonnes } else { 0.0 };
        total.blocks += r.blocks;
        total.tonnes += r.tonnes;
        total.metal += r.metal;
        total.volume += r.volume;
    }
    total.grade = if total.tonnes > 0.0 {
        total.metal / total.tonnes
    } else {
        0.0
    };
    rows.push(total);
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::BlockGeometry;

    /// 10 x 1 x 1 blocks of 10 m: grade = block index (0..9), density 2, all inside.
    fn model() -> BlockModel {
        let mut m = BlockModel::new(BlockGeometry {
            origin: [0.0; 3],
            size: [10.0, 10.0, 10.0],
            n: [10, 1, 1],
            rotation: 0.0,
        });
        m.domain_names = vec!["A".into(), "B".into()];
        for i in 0..10 {
            m.domain[i] = if i < 5 { 1 } else { 2 };
            m.fraction[i] = 1.0;
        }
        m.column_mut("au_ok")
            .iter_mut()
            .enumerate()
            .for_each(|(i, v)| *v = i as f32);
        m.column_mut("tonnage").iter_mut().for_each(|v| *v = 2000.0);
        m
    }

    #[test]
    fn summary_by_domain_and_total() {
        let rows = summary_by_domain(&model(), "au_ok", 0.0);
        assert_eq!(rows.len(), 3);
        assert_eq!((rows[0].blocks, rows[0].tonnes, rows[0].grade), (5, 10_000.0, 2.0));
        assert_eq!((rows[1].blocks, rows[1].grade), (5, 7.0));
        assert_eq!(
            (rows[2].domain.as_str(), rows[2].tonnes, rows[2].grade),
            ("Total", 20_000.0, 4.5)
        );
        // Cut-off 5: only blocks 5..9.
        let hi = summary_by_domain(&model(), "au_ok", 5.0);
        assert_eq!((hi[2].blocks, hi[2].grade, hi[0].blocks), (5, 7.0, 0));
    }

    #[test]
    fn unknown_domain_ids_do_not_panic() {
        let mut m = model();
        m.domain[9] = 7;
        let rows = summary_by_domain(&m, "au_ok", 0.0);
        assert_eq!(rows.last().unwrap().blocks, 9);
    }

    #[test]
    fn grade_tonnage_and_filters() {
        let gt = model_grade_tonnage(&model(), "au_ok", &Filter::all(), &[0.0, 5.0, 100.0]);
        assert_eq!(
            (gt[0].weight, gt[1].weight, gt[1].grade, gt[2].weight),
            (20_000.0, 10_000.0, 7.0, 0.0)
        );
        let b = model_grade_tonnage(
            &model(),
            "au_ok",
            &Filter {
                domain: 2,
                min_fraction: 0.0,
            },
            &[0.0],
        );
        assert_eq!(b[0].weight, 10_000.0);
        let s = block_summary(&model(), "au_ok", &Filter::all());
        assert_eq!((s.n, s.mean, s.min, s.max), (10, 4.5, 0.0, 9.0));
    }

    #[test]
    fn swath_slices_and_sample_swath() {
        let sw = swath(&model(), &["au_ok", "missing"], Axis::X, 20.0, &Filter::all());
        assert_eq!(sw.len(), 5);
        assert_eq!((sw[0].center, sw[0].blocks, sw[0].means[0]), (10.0, 2, 0.5));
        assert!(sw[0].means[1].is_nan());
        let samples: Vec<Sample> = (0..10)
            .map(|i| Sample {
                xyz: [i as f64 * 10.0 + 5.0, 5.0, 5.0],
                value: i as f64,
                length: 1.0,
                hole: 0,
                domain: 1,
            })
            .collect();
        let w = vec![1.0; 10];
        let ss = sample_swath(&samples, &w, Axis::X, 20.0, 0);
        assert_eq!(ss[0], (10.0, 0.5));
        let gt = sample_grade_tonnage(&samples, &w, &[5.0]);
        assert!((gt[0].weight - 0.5).abs() < 1e-12 && (gt[0].grade - 7.0).abs() < 1e-12);
    }
}
