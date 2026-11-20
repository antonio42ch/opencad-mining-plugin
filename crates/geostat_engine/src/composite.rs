//! Regularisation of sample intervals into composites of equal length.
//!
//! Composites never cross a domain boundary (a run of consecutive intervals
//! with the same domain is composited on its own, starting at its first
//! depth). The grade of a composite is the length-weighted mean of the
//! intervals that have a value; unsampled stretches do not dilute it.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Interval {
    pub from: f64,
    pub to: f64,
    /// `None` = unsampled / no value.
    pub value: Option<f64>,
    /// 1-based domain, 0 = none (skipped).
    pub domain: u32,
}

/// What to do with a final piece shorter than `min_fraction * length`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Remainder {
    /// Join it to the previous composite (which becomes longer).
    Merge,
    Discard,
    Keep,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CompositeConfig {
    pub length: f64,
    pub min_fraction: f64,
    pub remainder: Remainder,
    /// Composites whose sampled length is below this fraction of their
    /// length are dropped.
    pub min_coverage: f64,
}

impl Default for CompositeConfig {
    fn default() -> Self {
        CompositeConfig {
            length: 4.0,
            min_fraction: 0.5,
            remainder: Remainder::Merge,
            min_coverage: 0.5,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Composite {
    pub from: f64,
    pub to: f64,
    pub value: f64,
    pub domain: u32,
    /// Sampled length that contributed to the value.
    pub covered: f64,
}

impl Composite {
    pub fn length(&self) -> f64 {
        self.to - self.from
    }
}

fn weighted(intervals: &[Interval], a: f64, b: f64) -> Option<(f64, f64)> {
    let (mut sw, mut sv) = (0.0, 0.0);
    for i in intervals {
        let ov = i.to.min(b) - i.from.max(a);
        if ov > 1e-9 {
            if let Some(v) = i.value {
                sw += ov;
                sv += ov * v;
            }
        }
    }
    (sw > 1e-9).then(|| (sv / sw, sw))
}

/// Composites one hole. `intervals` must be sorted, non-overlapping.
pub fn composite_hole(intervals: &[Interval], cfg: &CompositeConfig) -> Vec<Composite> {
    let l = cfg.length;
    if !(l.is_finite() && l > 0.0) {
        return Vec::new();
    }
    let mut out: Vec<Composite> = Vec::new();
    let mut i = 0;
    while i < intervals.len() {
        let d = intervals[i].domain;
        let mut j = i;
        while j < intervals.len() && intervals[j].domain == d {
            j += 1;
        }
        if d != 0 {
            let run = &intervals[i..j];
            let (start, end) = (run[0].from, run.last().unwrap().to);
            let first_of_run = out.len();
            let mut a = start;
            while a < end - 1e-9 {
                let b = (a + l).min(end);
                let short = (b - a) < cfg.min_fraction * l - 1e-9;
                if short && b >= end - 1e-9 {
                    match cfg.remainder {
                        Remainder::Discard => break,
                        // Merge only into the composite that ends exactly where the
                        // short piece starts: never across a dropped (unsampled) stretch.
                        Remainder::Merge if out.len() > first_of_run && (out[out.len() - 1].to - a).abs() < 1e-9 => {
                            let prev = out.pop().unwrap();
                            let (a0, b0) = (prev.from, end);
                            match weighted(run, a0, b0) {
                                Some((v, w)) if w >= cfg.min_coverage * (b0 - a0) - 1e-9 => out.push(Composite {
                                    from: a0,
                                    to: b0,
                                    value: v,
                                    domain: d,
                                    covered: w,
                                }),
                                _ => out.push(prev),
                            }
                            break;
                        }
                        _ => {}
                    }
                }
                if let Some((v, w)) = weighted(run, a, b) {
                    if w >= cfg.min_coverage * (b - a) - 1e-9 {
                        out.push(Composite {
                            from: a,
                            to: b,
                            value: v,
                            domain: d,
                            covered: w,
                        });
                    }
                }
                a = b;
            }
        }
        i = j;
    }
    out
}

/// Caps values at `cap` (top-cut). Returns how many were capped.
pub fn cap_values(values: &mut [f64], cap: f64) -> usize {
    let mut n = 0;
    for v in values {
        if *v > cap {
            *v = cap;
            n += 1;
        }
    }
    n
}

#[cfg(test)]
mod tests {
    use super::*;

    fn iv(from: f64, to: f64, v: Option<f64>, d: u32) -> Interval {
        Interval {
            from,
            to,
            value: v,
            domain: d,
        }
    }

    #[test]
    fn preserves_metal_and_length() {
        // 1 m assays with varying grade over 0..10, composite 4 m: 4 + 4 + 2(<50%? 2/4 = 50% kept).
        let ivs: Vec<Interval> = (0..10)
            .map(|i| iv(i as f64, i as f64 + 1.0, Some(i as f64), 1))
            .collect();
        let c = composite_hole(&ivs, &CompositeConfig::default());
        let len: f64 = c.iter().map(|c| c.length()).sum();
        let metal: f64 = c.iter().map(|c| c.length() * c.value).sum();
        assert!((len - 10.0).abs() < 1e-9);
        assert!((metal - 45.0).abs() < 1e-9);
        assert_eq!(c.len(), 3);
        assert!((c[0].value - 1.5).abs() < 1e-12 && (c[2].value - 8.5).abs() < 1e-12);
    }

    #[test]
    fn remainder_modes() {
        let ivs: Vec<Interval> = (0..9).map(|i| iv(i as f64, i as f64 + 1.0, Some(1.0), 1)).collect();
        // 9 m: 4 + 4 + 1 (1 < 2 -> short).
        let mut cfg = CompositeConfig::default();
        let merged = composite_hole(&ivs, &cfg);
        assert_eq!(merged.len(), 2);
        assert!((merged[1].length() - 5.0).abs() < 1e-9);
        cfg.remainder = Remainder::Discard;
        assert_eq!(composite_hole(&ivs, &cfg).len(), 2);
        assert!((composite_hole(&ivs, &cfg)[1].length() - 4.0).abs() < 1e-9);
        cfg.remainder = Remainder::Keep;
        assert_eq!(composite_hole(&ivs, &cfg).len(), 3);
    }

    #[test]
    fn never_crosses_domains_and_skips_unassigned() {
        let ivs = vec![
            iv(0.0, 6.0, Some(2.0), 1),
            iv(6.0, 12.0, Some(8.0), 2),
            iv(12.0, 16.0, Some(5.0), 0),
        ];
        let c = composite_hole(&ivs, &CompositeConfig::default());
        assert!(c.iter().all(|c| c.domain != 0));
        for k in &c {
            let inside_1 = k.to <= 6.0 + 1e-9;
            let inside_2 = k.from >= 6.0 - 1e-9;
            assert!(inside_1 || inside_2, "{k:?} crosses the contact");
            assert_eq!(k.value, if inside_1 { 2.0 } else { 8.0 });
        }
    }

    #[test]
    fn unsampled_gaps_do_not_dilute_and_low_coverage_is_dropped() {
        let ivs = vec![iv(0.0, 2.0, Some(4.0), 1), iv(2.0, 4.0, None, 1), iv(4.0, 8.0, None, 1)];
        let c = composite_hole(&ivs, &CompositeConfig::default());
        // First composite 0..4 has 2 m sampled (50 %) with grade 4; second has none.
        assert_eq!(c.len(), 1);
        assert_eq!((c[0].value, c[0].covered), (4.0, 2.0));
    }

    #[test]
    fn short_tail_is_not_merged_across_a_dropped_stretch() {
        // 0-4 sampled, 4-8 unsampled (dropped), 8-9 short: must not become one 0-9 composite.
        let ivs = vec![
            iv(0.0, 4.0, Some(1.0), 1),
            iv(4.0, 8.0, None, 1),
            iv(8.0, 9.0, Some(9.0), 1),
        ];
        let c = composite_hole(&ivs, &CompositeConfig::default());
        assert!(c.iter().all(|k| k.length() <= 4.0 + 1e-9), "{c:?}");
        assert_eq!((c[0].from, c[0].to, c[0].value), (0.0, 4.0, 1.0));
        // Adjacent short tail still merges.
        let adj: Vec<Interval> = (0..9).map(|i| iv(i as f64, i as f64 + 1.0, Some(1.0), 1)).collect();
        assert!(
            (composite_hole(&adj, &CompositeConfig::default())
                .last()
                .unwrap()
                .length()
                - 5.0)
                .abs()
                < 1e-9
        );
    }

    #[test]
    fn capping() {
        let mut v = [1.0, 50.0, 7.0];
        assert_eq!(cap_values(&mut v, 10.0), 1);
        assert_eq!(v, [1.0, 10.0, 7.0]);
    }
}
