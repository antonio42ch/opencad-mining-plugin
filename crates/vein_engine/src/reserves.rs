//! Tonnage of a vein model (stage B3): dilution and recovery, blocks
//! between levels (galleries) and raises projected on the vein plane,
//! pillars, sampled sides per block and categories by an ordered rule table.

use crate::model::{Composite, Prism};

/// Dilution and recovery applied to every prism.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Dilution {
    /// Minimum mining width (m): thinner prisms are widened half on each wall.
    pub mmw: f64,
    /// Overbreak on the hanging wall and on the footwall (m).
    pub ob_hw: f64,
    pub ob_fw: f64,
    /// Grade of the diluting rock on each wall.
    pub grade_hw: f64,
    pub grade_fw: f64,
    pub density_waste: f64,
    /// Mining recovery, 0..1.
    pub recovery: f64,
}

impl Default for Dilution {
    fn default() -> Self {
        Dilution {
            mmw: 0.8,
            ob_hw: 0.1,
            ob_fw: 0.05,
            grade_hw: 0.0,
            grade_fw: 0.0,
            density_waste: 2.6,
            recovery: 0.9,
        }
    }
}

/// Diluted figures of one prism.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Diluted {
    pub width: f64,
    pub tonnes: f64,
    pub grade: f64,
    pub waste: f64,
    pub recovered: f64,
}

pub fn dilute(p: &Prism, d: &Dilution) -> Diluted {
    let t = p.thickness.max(0.0);
    let rho = if p.volume > 0.0 { p.tonnes / p.volume } else { 0.0 };
    let extra = (d.mmw - t).max(0.0) / 2.0;
    let (hw, fw) = (extra + d.ob_hw.max(0.0), extra + d.ob_fw.max(0.0));
    let ore = p.area * t * rho;
    let (whw, wfw) = (p.area * hw * d.density_waste, p.area * fw * d.density_waste);
    let tonnes = ore + whw + wfw;
    let grade = if tonnes > 0.0 {
        (ore * p.grade + whw * d.grade_hw + wfw * d.grade_fw) / tonnes
    } else {
        0.0
    };
    Diluted {
        width: t + hw + fw,
        tonnes,
        grade,
        waste: whw + wfw,
        recovered: tonnes * d.recovery.clamp(0.0, 1.0),
    }
}

/// Polyline on the vein plane `(u, v)`.
pub type Line2 = Vec<[f64; 2]>;

/// `v` of a level line at `u` (linear, constant beyond its ends).
pub fn v_at(line: &[[f64; 2]], u: f64) -> f64 {
    interp(line, u, 0, 1)
}

/// `u` of a raise line at `v`.
pub fn u_at(line: &[[f64; 2]], v: f64) -> f64 {
    interp(line, v, 1, 0)
}

fn interp(line: &[[f64; 2]], x: f64, a: usize, b: usize) -> f64 {
    let mut pts: Vec<[f64; 2]> = line.to_vec();
    pts.sort_by(|p, q| p[a].total_cmp(&q[a]));
    match pts.len() {
        0 => f64::NAN,
        1 => pts[0][b],
        _ => {
            if x <= pts[0][a] {
                return pts[0][b];
            }
            for w in pts.windows(2) {
                if x <= w[1][a] {
                    let span = w[1][a] - w[0][a];
                    let t = if span.abs() < 1e-12 { 0.0 } else { (x - w[0][a]) / span };
                    return w[0][b] + t * (w[1][b] - w[0][b]);
                }
            }
            pts[pts.len() - 1][b]
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Workings {
    pub levels: Vec<Line2>,
    pub raises: Vec<Line2>,
}

impl Workings {
    /// Levels bottom-up and raises left to right (by their mean coordinate).
    pub fn sorted(mut self) -> Workings {
        let mean = |l: &Line2, k: usize| l.iter().map(|p| p[k]).sum::<f64>() / l.len().max(1) as f64;
        self.levels.retain(|l| !l.is_empty());
        self.raises.retain(|l| !l.is_empty());
        self.levels.sort_by(|a, b| mean(a, 1).total_cmp(&mean(b, 1)));
        self.raises.sort_by(|a, b| mean(a, 0).total_cmp(&mean(b, 0)));
        self
    }

    /// Panel of a point: (levels below it, raises left of it).
    pub fn panel(&self, u: f64, v: f64) -> (usize, usize) {
        (
            self.levels.iter().filter(|l| v_at(l, u) <= v).count(),
            self.raises.iter().filter(|r| u_at(r, v) <= u).count(),
        )
    }

    /// Name of a panel: `L1-L2/R1-R2`, `<L1`, `>L3`, ... (`all` without workings).
    pub fn name(&self, k: usize, m: usize) -> String {
        let part = |i: usize, n: usize, c: char| -> Option<String> {
            match (n, i) {
                (0, _) => None,
                (_, 0) => Some(format!("<{c}1")),
                (_, i) if i == n => Some(format!(">{c}{n}")),
                (_, i) => Some(format!("{c}{i}-{c}{}", i + 1)),
            }
        };
        match (part(k, self.levels.len(), 'L'), part(m, self.raises.len(), 'R')) {
            (Some(a), Some(b)) => format!("{a}/{b}"),
            (Some(a), None) | (None, Some(a)) => a,
            (None, None) => "all".into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BlockParams {
    /// Sill pillar left above each level (m along the plane, 0 = none).
    pub level_pillar: f64,
    /// Total width of the pillar centred on each raise (m, 0 = none).
    pub raise_pillar: f64,
    /// A composite within this distance of a working samples that side (m).
    pub side_tol: f64,
    /// Composites needed on a side to count it as sampled.
    pub side_min: usize,
}

impl Default for BlockParams {
    fn default() -> Self {
        BlockParams {
            level_pillar: 0.0,
            raise_pillar: 0.0,
            side_tol: 3.0,
            side_min: 1,
        }
    }
}

/// One rule of the category table: every condition given must hold.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Rule {
    pub category: String,
    pub min_sides: Option<usize>,
    pub max_dist: Option<f64>,
    pub min_samples: Option<usize>,
    pub min_grade: Option<f64>,
}

/// Values a rule is checked against (per block).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Attrs {
    pub sides: usize,
    /// Tonnage-weighted mean distance of the prisms to the samples (m).
    pub dist: f64,
    pub samples: usize,
    /// Diluted grade.
    pub grade: f64,
}

/// Index of the first rule met, and why each earlier rule failed.
pub fn classify(rules: &[Rule], a: &Attrs) -> (Option<usize>, Vec<String>) {
    let mut why = Vec::new();
    for (i, r) in rules.iter().enumerate() {
        let mut fails = Vec::new();
        if let Some(x) = r.min_sides.filter(|x| a.sides < *x) {
            fails.push(format!("sides {} < {x}", a.sides));
        }
        if let Some(x) = r.max_dist.filter(|x| a.dist.is_nan() || a.dist > *x) {
            fails.push(format!("dist {:.1} > {x}", a.dist));
        }
        if let Some(x) = r.min_samples.filter(|x| a.samples < *x) {
            fails.push(format!("samples {} < {x}", a.samples));
        }
        if let Some(x) = r.min_grade.filter(|x| a.grade.is_nan() || a.grade < *x) {
            fails.push(format!("grade {:.3} < {x}", a.grade));
        }
        if fails.is_empty() {
            return (Some(i), why);
        }
        why.push(format!("{}: {}", r.category, fails.join(", ")));
    }
    (None, why)
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Block {
    pub name: String,
    pub level: usize,
    pub raise: usize,
    /// Sampled state of the lower, upper, left and right sides (`None` = no working there).
    pub sides: [Option<bool>; 4],
    pub sampled: usize,
    pub samples: usize,
    pub dist: f64,
    pub prisms: usize,
    pub area: f64,
    pub thickness: f64,
    pub tonnes: f64,
    pub grade: f64,
    pub diluted_tonnes: f64,
    pub diluted_grade: f64,
    pub width: f64,
    pub recovered: f64,
    pub pillar_tonnes: f64,
    pub pillar_grade: f64,
    pub category: Option<usize>,
    pub why: Vec<String>,
}

/// What became of each prism: its block and whether it is pillar.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Assignment {
    pub block: usize,
    pub pillar: bool,
}

/// Groups the prisms into blocks, dilutes them and classifies the blocks.
pub fn cubicate(
    prisms: &[Prism],
    comps: &[Composite],
    w: &Workings,
    d: &Dilution,
    bp: &BlockParams,
    rules: &[Rule],
) -> (Vec<Block>, Vec<Assignment>) {
    let mut index: std::collections::BTreeMap<(usize, usize), usize> = Default::default();
    let mut blocks: Vec<Block> = Vec::new();
    let mut assign = Vec::with_capacity(prisms.len());
    // Accumulators per block: (dist*t, umin, umax, vmin, vmax, width*t).
    let mut acc: Vec<(f64, f64, f64, f64, f64, f64)> = Vec::new();
    for p in prisms {
        let (k, m) = w.panel(p.u, p.v);
        let b = *index.entry((k, m)).or_insert_with(|| {
            blocks.push(Block {
                name: w.name(k, m),
                level: k,
                raise: m,
                ..Default::default()
            });
            acc.push((0.0, f64::MAX, f64::MIN, f64::MAX, f64::MIN, 0.0));
            blocks.len() - 1
        });
        let pillar = (bp.level_pillar > 0.0 && k > 0 && {
            let dv = p.v - v_at(&w.levels[k - 1], p.u);
            (0.0..bp.level_pillar).contains(&dv)
        }) || (bp.raise_pillar > 0.0
            && w.raises
                .iter()
                .any(|r| (p.u - u_at(r, p.v)).abs() < bp.raise_pillar / 2.0));
        assign.push(Assignment { block: b, pillar });
        let bl = &mut blocks[b];
        let a = &mut acc[b];
        a.1 = a.1.min(p.u);
        a.2 = a.2.max(p.u);
        a.3 = a.3.min(p.v);
        a.4 = a.4.max(p.v);
        if pillar {
            bl.pillar_grade += p.grade * p.tonnes;
            bl.pillar_tonnes += p.tonnes;
            continue;
        }
        let dl = dilute(p, d);
        bl.prisms += 1;
        bl.area += p.area;
        bl.thickness += p.thickness * p.area;
        bl.tonnes += p.tonnes;
        bl.grade += p.grade * p.tonnes;
        bl.diluted_tonnes += dl.tonnes;
        bl.diluted_grade += dl.grade * dl.tonnes;
        bl.recovered += dl.recovered;
        a.0 += p.dist * p.tonnes;
        a.5 += dl.width * p.area;
    }
    for (b, bl) in blocks.iter_mut().enumerate() {
        let a = acc[b];
        if bl.tonnes > 0.0 {
            bl.grade /= bl.tonnes;
            bl.dist = a.0 / bl.tonnes;
        }
        if bl.area > 0.0 {
            bl.thickness /= bl.area;
            bl.width = a.5 / bl.area;
        }
        if bl.diluted_tonnes > 0.0 {
            bl.diluted_grade /= bl.diluted_tonnes;
        }
        if bl.pillar_tonnes > 0.0 {
            bl.pillar_grade /= bl.pillar_tonnes;
        }
        // Sides: the workings that bound the block, sampled when enough
        // composites lie on them within the block's span.
        let (k, m) = (bl.level, bl.raise);
        let tol = bp.side_tol.max(0.0);
        let span_u = (a.1 - tol, a.2 + tol);
        let span_v = (a.3 - tol, a.4 + tol);
        let on_level = |l: &Line2| {
            comps
                .iter()
                .filter(|c| (c.u >= span_u.0 && c.u <= span_u.1) && (c.v - v_at(l, c.u)).abs() <= tol)
                .count()
        };
        let on_raise = |r: &Line2| {
            comps
                .iter()
                .filter(|c| (c.v >= span_v.0 && c.v <= span_v.1) && (c.u - u_at(r, c.v)).abs() <= tol)
                .count()
        };
        let counts = [
            (k > 0).then(|| on_level(&w.levels[k - 1])),
            (k < w.levels.len()).then(|| on_level(&w.levels[k])),
            (m > 0).then(|| on_raise(&w.raises[m - 1])),
            (m < w.raises.len()).then(|| on_raise(&w.raises[m])),
        ];
        bl.sides = counts.map(|c| c.map(|n| n >= bp.side_min.max(1)));
        bl.sampled = bl.sides.iter().filter(|s| **s == Some(true)).count();
        let inside = comps.iter().filter(|c| w.panel(c.u, c.v) == (k, m)).count();
        bl.samples = inside.max(counts.iter().flatten().sum::<usize>());
        let (cat, why) = classify(
            rules,
            &Attrs {
                sides: bl.sampled,
                dist: bl.dist,
                samples: bl.samples,
                grade: bl.diluted_grade,
            },
        );
        bl.category = cat;
        bl.why = why;
    }
    (blocks, assign)
}

/// Totals per category (index into `rules`; the last entry is unclassified).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Total {
    pub blocks: usize,
    pub tonnes: f64,
    pub grade: f64,
    pub diluted_tonnes: f64,
    pub diluted_grade: f64,
    pub recovered: f64,
}

pub fn totals(blocks: &[Block], n_rules: usize) -> Vec<Total> {
    let mut t = vec![Total::default(); n_rules + 1];
    for b in blocks {
        let x = &mut t[b.category.unwrap_or(n_rules)];
        x.blocks += 1;
        x.tonnes += b.tonnes;
        x.grade += b.grade * b.tonnes;
        x.diluted_tonnes += b.diluted_tonnes;
        x.diluted_grade += b.diluted_grade * b.diluted_tonnes;
        x.recovered += b.recovered;
    }
    for x in t.iter_mut() {
        if x.tonnes > 0.0 {
            x.grade /= x.tonnes;
        }
        if x.diluted_tonnes > 0.0 {
            x.diluted_grade /= x.diluted_tonnes;
        }
    }
    t
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prism(u: f64, v: f64, t: f64, g: f64) -> Prism {
        let area = 25.0;
        Prism {
            i: 0,
            j: 0,
            u,
            v,
            centre: [0.0; 3],
            thickness: t,
            area,
            volume: area * t,
            tonnes: area * t * 2.8,
            grade: g,
            acc: g * t,
            dist: 5.0,
            n: 4,
            variance: 0.1,
            edited: false,
            corners: [[0.0; 3]; 8],
        }
    }

    #[test]
    fn dilution_widens_to_the_mmw_and_adds_overbreak() {
        let d = Dilution {
            mmw: 1.0,
            ob_hw: 0.2,
            ob_fw: 0.1,
            grade_hw: 1.0,
            grade_fw: 0.0,
            density_waste: 2.5,
            recovery: 0.9,
        };
        let x = dilute(&prism(0.0, 0.0, 0.4, 10.0), &d);
        // 0.4 m vein -> 0.3 m added on each wall, plus overbreak.
        assert!((x.width - (0.4 + 0.5 + 0.4)).abs() < 1e-12);
        let ore = 25.0 * 0.4 * 2.8;
        let (whw, wfw) = (25.0 * 0.5 * 2.5, 25.0 * 0.4 * 2.5);
        assert!((x.tonnes - (ore + whw + wfw)).abs() < 1e-9);
        assert!((x.grade - (ore * 10.0 + whw * 1.0) / (ore + whw + wfw)).abs() < 1e-9);
        assert!((x.recovered - 0.9 * x.tonnes).abs() < 1e-9);
        // A wide vein only takes the overbreak.
        let y = dilute(&prism(0.0, 0.0, 2.0, 10.0), &d);
        assert!((y.width - 2.3).abs() < 1e-12);
    }

    #[test]
    fn lines_interpolate_and_clamp() {
        let l = vec![[0.0, 10.0], [20.0, 12.0]];
        assert!((v_at(&l, 10.0) - 11.0).abs() < 1e-12 && v_at(&l, -5.0) == 10.0 && v_at(&l, 50.0) == 12.0);
        let r = vec![[5.0, 0.0], [7.0, 40.0]];
        assert!((u_at(&r, 20.0) - 6.0).abs() < 1e-12);
    }

    fn workings() -> Workings {
        Workings {
            levels: vec![vec![[-100.0, 50.0], [100.0, 50.0]], vec![[-100.0, 0.0], [100.0, 0.0]]],
            raises: vec![vec![[0.0, -10.0], [0.0, 60.0]]],
        }
        .sorted()
    }

    fn rules() -> Vec<Rule> {
        vec![
            Rule {
                category: "Medido".into(),
                min_sides: Some(3),
                max_dist: Some(10.0),
                ..Default::default()
            },
            Rule {
                category: "Indicado".into(),
                min_sides: Some(2),
                ..Default::default()
            },
            Rule {
                category: "Inferido".into(),
                ..Default::default()
            },
        ]
    }

    #[test]
    fn blocks_between_levels_and_raises_with_sides_and_categories() {
        let w = workings();
        assert_eq!(w.name(1, 0), "L1-L2/<R1");
        assert_eq!(w.name(1, 1), "L1-L2/>R1");
        assert_eq!(w.name(0, 0), "<L1/<R1");
        let mut prisms = Vec::new();
        for i in 0..8 {
            for j in 0..10 {
                prisms.push(prism(-40.0 + i as f64 * 10.0 + 5.0, j as f64 * 5.0 + 2.5, 1.0, 5.0));
            }
        }
        // Channels on both levels on the right panel, and along the raise; only the lower level on the left.
        let ch = |u: f64, v: f64| Composite {
            hole: format!("{u},{v}"),
            u,
            v,
            p: [0.0; 3],
            thickness: 1.0,
            grade: 5.0,
            acc: 5.0,
        };
        let mut comps: Vec<Composite> = (0..4)
            .flat_map(|k| [ch(5.0 + k as f64 * 10.0, 0.5), ch(5.0 + k as f64 * 10.0, 49.5)])
            .collect();
        comps.push(ch(0.5, 25.0));
        comps.push(ch(-25.0, 0.5));
        let (blocks, assign) = cubicate(
            &prisms,
            &comps,
            &w,
            &Dilution::default(),
            &BlockParams::default(),
            &rules(),
        );
        assert_eq!(blocks.len(), 2);
        assert_eq!(assign.len(), prisms.len());
        let right = blocks.iter().find(|b| b.name == "L1-L2/>R1").unwrap();
        let left = blocks.iter().find(|b| b.name == "L1-L2/<R1").unwrap();
        // Right: lower, upper and the raise on its left are sampled; no working on its right.
        assert_eq!(right.sides, [Some(true), Some(true), Some(true), None]);
        assert_eq!((right.sampled, right.category), (3, Some(0)));
        // Left: the lower level and the raise (on its right) are sampled.
        assert_eq!(left.sides, [Some(true), Some(false), None, Some(true)]);
        assert_eq!((left.sampled, left.category), (2, Some(1)));
        assert!(left.why[0].starts_with("Medido: sides 2 < 3"));
        assert!(
            (right.tonnes - 40.0 * 25.0 * 2.8).abs() < 1e-6
                && right.diluted_tonnes > right.tonnes
                && right.recovered < right.diluted_tonnes
        );
        let t = totals(&blocks, 3);
        assert_eq!((t[0].blocks, t[1].blocks, t[3].blocks), (1, 1, 0));
    }

    #[test]
    fn pillars_are_kept_apart() {
        let w = workings();
        let prisms: Vec<Prism> = (0..10)
            .map(|j| prism(12.5, j as f64 * 5.0 + 2.5, 1.0, 5.0))
            .chain([prism(1.0, 25.0, 1.0, 5.0)])
            .collect();
        let bp = BlockParams {
            level_pillar: 5.0,
            raise_pillar: 4.0,
            ..Default::default()
        };
        let (blocks, assign) = cubicate(&prisms, &[], &w, &Dilution::default(), &bp, &rules());
        // v = 2.5 is within 5 m above L1; u = 1 is within 2 m of the raise.
        assert_eq!(assign.iter().filter(|a| a.pillar).count(), 2);
        let pillar_t: f64 = blocks.iter().map(|b| b.pillar_tonnes).sum();
        assert!((pillar_t - 2.0 * 25.0 * 2.8).abs() < 1e-9);
        // No samples: only the last rule (no conditions) applies.
        assert!(blocks.iter().all(|b| b.category == Some(2)));
    }

    #[test]
    fn rules_are_checked_in_order() {
        let r = rules();
        assert_eq!(
            classify(
                &r,
                &Attrs {
                    sides: 4,
                    dist: 5.0,
                    samples: 9,
                    grade: 3.0
                }
            )
            .0,
            Some(0)
        );
        let (c, why) = classify(
            &r,
            &Attrs {
                sides: 4,
                dist: 15.0,
                samples: 9,
                grade: 3.0,
            },
        );
        assert_eq!(c, Some(1));
        assert_eq!(why, vec!["Medido: dist 15.0 > 10".to_string()]);
        let strict = vec![Rule {
            category: "Reserva".into(),
            min_grade: Some(4.0),
            min_samples: Some(2),
            ..Default::default()
        }];
        let (c, why) = classify(
            &strict,
            &Attrs {
                sides: 0,
                dist: 1.0,
                samples: 1,
                grade: 3.0,
            },
        );
        assert_eq!(c, None);
        assert_eq!(why, vec!["Reserva: samples 1 < 2, grade 3.000 < 4".to_string()]);
    }
}
