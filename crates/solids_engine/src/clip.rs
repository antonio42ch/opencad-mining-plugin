//! Clipping of section polylines to a window of the section's 2D frame.
//!
//! A terrain cut by a vertical plane is a polyline as long as the terrain is
//! wide; the section sheets only need the stretch around the solids. The window
//! may leave an axis unbounded (a vertical section is clipped along the
//! section, never in height).

use mining_core::Vec2;

use crate::cut::{Plane, SectionLoop};

/// Limits on the 2D axes of a plane; `None` leaves the axis unbounded.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Window {
    pub x: Option<(f64, f64)>,
    pub y: Option<(f64, f64)>,
}

impl Window {
    pub fn contains(&self, p: Vec2) -> bool {
        let inside = |v: f64, r: Option<(f64, f64)>| r.is_none_or(|(a, b)| v >= a - 1e-9 && v <= b + 1e-9);
        inside(p[0], self.x) && inside(p[1], self.y)
    }

    pub fn is_unbounded(&self) -> bool {
        self.x.is_none() && self.y.is_none()
    }
}

/// Part of the segment `a`-`b` inside the window (Liang–Barsky), as the two end points.
fn clip_segment(a: Vec2, b: Vec2, w: &Window) -> Option<(Vec2, Vec2)> {
    let (mut t0, mut t1) = (0.0f64, 1.0f64);
    let d = [b[0] - a[0], b[1] - a[1]];
    for (axis, range) in [(0usize, w.x), (1usize, w.y)] {
        let Some((lo, hi)) = range else { continue };
        for (p, q) in [(-d[axis], a[axis] - lo), (d[axis], hi - a[axis])] {
            if p.abs() < 1e-15 {
                if q < 0.0 {
                    return None;
                }
            } else {
                let r = q / p;
                if p < 0.0 {
                    t0 = t0.max(r);
                } else {
                    t1 = t1.min(r);
                }
            }
        }
    }
    (t0 <= t1).then(|| {
        let at = |t: f64| [a[0] + t * d[0], a[1] + t * d[1]];
        (at(t0), at(t1))
    })
}

fn same(a: Vec2, b: Vec2) -> bool {
    (a[0] - b[0]).abs() < 1e-9 && (a[1] - b[1]).abs() < 1e-9
}

/// The loops cut to the window. A closed loop entirely inside stays as it is;
/// anything else comes out as open pieces (a closed loop cut by the window loses its closure).
pub fn clip(loops: &[SectionLoop], plane: &Plane, w: &Window) -> Vec<SectionLoop> {
    if w.is_unbounded() {
        return loops.to_vec();
    }
    let mut out = Vec::new();
    for l in loops {
        let p = &l.points_2d;
        if p.len() < 2 {
            continue;
        }
        if l.closed && p.iter().all(|q| w.contains(*q)) {
            out.push(l.clone());
            continue;
        }
        let n = if l.closed { p.len() } else { p.len() - 1 };
        let mut pieces: Vec<Vec<Vec2>> = Vec::new();
        let mut cur: Vec<Vec2> = Vec::new();
        for i in 0..n {
            let (a, b) = (p[i], p[(i + 1) % p.len()]);
            match clip_segment(a, b, w) {
                None => {
                    if cur.len() > 1 {
                        pieces.push(std::mem::take(&mut cur));
                    }
                    cur.clear();
                }
                Some((ca, cb)) => {
                    if cur.last().is_some_and(|q| !same(*q, ca)) {
                        if cur.len() > 1 {
                            pieces.push(std::mem::take(&mut cur));
                        }
                        cur.clear();
                    }
                    if cur.is_empty() {
                        cur.push(ca);
                    }
                    if !same(*cur.last().unwrap(), cb) {
                        cur.push(cb);
                    }
                }
            }
        }
        if cur.len() > 1 {
            pieces.push(cur);
        }
        // A closed loop whose last piece runs into the first one is a single piece.
        if l.closed && pieces.len() > 1 {
            let (first_start, last_end) = (pieces[0][0], *pieces[pieces.len() - 1].last().unwrap());
            if same(first_start, last_end) {
                let first = pieces.remove(0);
                pieces.last_mut().unwrap().extend(first.into_iter().skip(1));
            }
        }
        for piece in pieces {
            out.push(SectionLoop {
                points: piece.iter().map(|q| plane.to_3d(*q)).collect(),
                points_2d: piece,
                closed: false,
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plane() -> Plane {
        // The vertical plane y = 0: u along +x, v up.
        Plane {
            origin: [0.0; 3],
            normal: [0.0, -1.0, 0.0],
            u: [1.0, 0.0, 0.0],
            v: [0.0, 0.0, 1.0],
        }
    }

    fn line(pts: &[[f64; 2]], closed: bool) -> SectionLoop {
        let p = plane();
        SectionLoop {
            points: pts.iter().map(|q| p.to_3d(*q)).collect(),
            points_2d: pts.to_vec(),
            closed,
        }
    }

    #[test]
    fn a_profile_is_cut_along_the_section_and_never_in_height() {
        let p = plane();
        let profile = line(&[[-100.0, 500.0], [0.0, 510.0], [100.0, 520.0]], false);
        let w = Window {
            x: Some((-20.0, 50.0)),
            y: None,
        };
        let c = clip(&[profile], &p, &w);
        assert_eq!(c.len(), 1);
        let q = &c[0].points_2d;
        assert!((q[0][0] + 20.0).abs() < 1e-9 && (q.last().unwrap()[0] - 50.0).abs() < 1e-9);
        // Heights follow the profile (500 + 0.1 (x + 100) on the first leg, 510 + 0.1 x on the second).
        assert!((q[0][1] - 508.0).abs() < 1e-9 && (q.last().unwrap()[1] - 515.0).abs() < 1e-9);
        // 3D points agree with the 2D ones.
        assert!(c[0]
            .points
            .iter()
            .zip(q)
            .all(|(a, b)| (a[0] - b[0]).abs() < 1e-9 && (a[2] - b[1]).abs() < 1e-9));
    }

    #[test]
    fn outside_the_window_nothing_remains_and_unbounded_keeps_everything() {
        let p = plane();
        let l = line(&[[0.0, 0.0], [10.0, 0.0]], false);
        assert!(clip(
            std::slice::from_ref(&l),
            &p,
            &Window {
                x: Some((20.0, 30.0)),
                y: None
            }
        )
        .is_empty());
        assert_eq!(clip(std::slice::from_ref(&l), &p, &Window::default()), vec![l]);
    }

    #[test]
    fn closed_loops_inside_stay_closed_and_cut_ones_become_open() {
        let p = plane();
        let sq = line(&[[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]], true);
        let all = Window {
            x: Some((-5.0, 15.0)),
            y: Some((-5.0, 15.0)),
        };
        let kept = clip(std::slice::from_ref(&sq), &p, &all);
        assert_eq!(kept.len(), 1);
        assert!(kept[0].closed);
        let half = Window {
            x: Some((-5.0, 5.0)),
            y: Some((-5.0, 15.0)),
        };
        let cut = clip(&[sq], &p, &half);
        assert_eq!(cut.len(), 1, "the pieces of a wrapped loop join: {cut:?}");
        assert!(!cut[0].closed);
        assert!(cut[0].points_2d.iter().all(|q| q[0] <= 5.0 + 1e-9));
        // Top edge from the window to the corner (5 m), the whole left side (10 m), the bottom edge to the window (5 m).
        let len: f64 = cut[0]
            .points_2d
            .windows(2)
            .map(|w| ((w[1][0] - w[0][0]).powi(2) + (w[1][1] - w[0][1]).powi(2)).sqrt())
            .sum();
        assert!((len - 20.0).abs() < 1e-9, "{len}");
    }

    #[test]
    fn a_profile_that_leaves_and_returns_gives_two_pieces() {
        let p = plane();
        let l = line(
            &[
                [0.0, 0.0],
                [10.0, 0.0],
                [10.0, 20.0],
                [20.0, 20.0],
                [20.0, 0.0],
                [30.0, 0.0],
            ],
            false,
        );
        let c = clip(
            &[l],
            &p,
            &Window {
                x: None,
                y: Some((-1.0, 5.0)),
            },
        );
        assert_eq!(c.len(), 2);
    }
}
