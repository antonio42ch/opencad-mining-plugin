//! Minimal SVG charts for the panels. Pure functions; colours are mid-tones
//! that read on both light and dark themes and the background is transparent.

use std::fmt::Write;

const PALETTE: [&str; 6] = ["#4a9eff", "#ff6b4a", "#3fb950", "#d29922", "#bc6bff", "#39c5cf"];
const AXIS: &str = "#888888";
const GRID: &str = "#88888833";

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Style {
    Line,
    /// Dots (experimental points).
    Dots,
    Bars,
    /// Dashed line (reference, e.g. the sill).
    Dashed,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Series {
    pub name: String,
    pub points: Vec<(f64, f64)>,
    pub style: Style,
    /// Palette index.
    pub color: usize,
}

impl Series {
    pub fn new(name: impl Into<String>, points: Vec<(f64, f64)>, style: Style, color: usize) -> Self {
        Series {
            name: name.into(),
            points,
            style,
            color,
        }
    }
}

pub struct Chart<'a> {
    pub title: &'a str,
    pub x_label: &'a str,
    pub y_label: &'a str,
    pub series: &'a [Series],
    pub width: f64,
    pub height: f64,
    /// Force the Y axis to start at 0.
    pub y_from_zero: bool,
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

/// "Nice" tick step for a range.
fn nice_step(range: f64, target: usize) -> f64 {
    let raw = range / target.max(1) as f64;
    let p = 10f64.powf(raw.log10().floor());
    let m = raw / p;
    p * if m < 1.5 {
        1.0
    } else if m < 3.5 {
        2.0
    } else if m < 7.5 {
        5.0
    } else {
        10.0
    }
}

fn fmt_tick(v: f64) -> String {
    if v.abs() >= 1000.0 || (v != 0.0 && v.abs() < 0.01) {
        format!("{v:.3e}").replace("e0", "e")
    } else if (v - v.round()).abs() < 1e-9 {
        format!("{v:.0}")
    } else {
        format!("{v:.2}")
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    }
}

pub fn render(c: &Chart) -> String {
    let finite: Vec<(f64, f64)> = c
        .series
        .iter()
        .flat_map(|s| s.points.iter().copied())
        .filter(|p| p.0.is_finite() && p.1.is_finite())
        .collect();
    let (w, h) = (c.width, c.height);
    let (ml, mr, mt, mb) = (52.0, 12.0, 24.0, 36.0);
    let mut o = String::new();
    let _ = write!(
        o,
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {w} {h}" width="{w}" height="{h}" font-family="sans-serif" font-size="10">"#
    );
    let _ = write!(
        o,
        r##"<text x="{}" y="14" fill="{AXIS}" font-size="11" font-weight="bold">{}</text>"##,
        ml,
        esc(c.title)
    );
    if finite.is_empty() {
        let _ = write!(
            o,
            r#"<text x="{}" y="{}" fill="{AXIS}" text-anchor="middle">-</text></svg>"#,
            w / 2.0,
            h / 2.0
        );
        return o;
    }
    let (mut x0, mut x1) = finite
        .iter()
        .fold((f64::MAX, f64::MIN), |(a, b), p| (a.min(p.0), b.max(p.0)));
    let (mut y0, mut y1) = finite
        .iter()
        .fold((f64::MAX, f64::MIN), |(a, b), p| (a.min(p.1), b.max(p.1)));
    if c.y_from_zero {
        y0 = y0.min(0.0);
    }
    if x1 - x0 < 1e-12 {
        (x0, x1) = (x0 - 0.5, x1 + 0.5);
    }
    if y1 - y0 < 1e-12 {
        (y0, y1) = (y0 - 0.5, y1 + 0.5);
    }
    let (sx, sy) = (nice_step(x1 - x0, 6), nice_step(y1 - y0, 5));
    let (x0, y0) = (
        (x0 / sx).floor() * sx,
        if c.y_from_zero && y0 >= 0.0 {
            0.0
        } else {
            (y0 / sy).floor() * sy
        },
    );
    let (x1, y1) = ((x1 / sx).ceil() * sx, (y1 / sy).ceil() * sy);
    let px = |x: f64| ml + (x - x0) / (x1 - x0) * (w - ml - mr);
    let py = |y: f64| h - mb - (y - y0) / (y1 - y0) * (h - mt - mb);
    let mut t = x0;
    while t <= x1 + sx * 1e-6 {
        let _ = write!(
            o,
            r#"<line x1="{0:.1}" y1="{1}" x2="{0:.1}" y2="{2}" stroke="{GRID}"/><text x="{0:.1}" y="{3}" fill="{AXIS}" text-anchor="middle">{4}</text>"#,
            px(t),
            mt,
            h - mb,
            h - mb + 12.0,
            fmt_tick(t)
        );
        t += sx;
    }
    let mut t = y0;
    while t <= y1 + sy * 1e-6 {
        let _ = write!(
            o,
            r#"<line x1="{1}" y1="{0:.1}" x2="{2}" y2="{0:.1}" stroke="{GRID}"/><text x="{3}" y="{4:.1}" fill="{AXIS}" text-anchor="end">{5}</text>"#,
            py(t),
            ml,
            w - mr,
            ml - 4.0,
            py(t) + 3.0,
            fmt_tick(t)
        );
        t += sy;
    }
    let _ = write!(
        o,
        r#"<rect x="{ml}" y="{mt}" width="{}" height="{}" fill="none" stroke="{AXIS}"/>"#,
        w - ml - mr,
        h - mt - mb
    );
    let bar_w = c
        .series
        .iter()
        .filter(|s| s.style == Style::Bars)
        .flat_map(|s| s.points.windows(2).map(|p| (p[1].0 - p[0].0).abs()))
        .fold(f64::MAX, f64::min);
    let bar_px = if bar_w.is_finite() && bar_w < f64::MAX {
        (px(x0 + bar_w) - px(x0)).max(1.0) * 0.85
    } else {
        8.0
    };
    for s in c.series {
        let col = PALETTE[s.color % PALETTE.len()];
        let pts: Vec<(f64, f64)> = s
            .points
            .iter()
            .copied()
            .filter(|p| p.0.is_finite() && p.1.is_finite())
            .collect();
        match s.style {
            Style::Line | Style::Dashed if pts.len() > 1 => {
                let d: Vec<String> = pts
                    .iter()
                    .enumerate()
                    .map(|(i, p)| format!("{}{:.1},{:.1}", if i == 0 { "M" } else { "L" }, px(p.0), py(p.1)))
                    .collect();
                let dash = if s.style == Style::Dashed {
                    r#" stroke-dasharray="4 3""#
                } else {
                    ""
                };
                let _ = write!(
                    o,
                    r#"<path d="{}" fill="none" stroke="{col}" stroke-width="1.6"{dash}/>"#,
                    d.join(" ")
                );
            }
            Style::Dots | Style::Line | Style::Dashed => {
                for p in &pts {
                    let _ = write!(
                        o,
                        r#"<circle cx="{:.1}" cy="{:.1}" r="2.6" fill="{col}"/>"#,
                        px(p.0),
                        py(p.1)
                    );
                }
            }
            Style::Bars => {
                for p in &pts {
                    let (top, base) = (py(p.1), py(y0.max(0.0)));
                    let _ = write!(
                        o,
                        r#"<rect x="{:.1}" y="{:.1}" width="{bar_px:.1}" height="{:.1}" fill="{col}" fill-opacity="0.7"/>"#,
                        px(p.0) - bar_px / 2.0,
                        top.min(base),
                        (base - top).abs()
                    );
                }
            }
        }
    }
    // Legend + axis labels.
    for (i, s) in c.series.iter().filter(|s| !s.name.is_empty()).enumerate() {
        let _ = write!(
            o,
            r#"<rect x="{}" y="{}" width="8" height="8" fill="{}"/><text x="{}" y="{}" fill="{AXIS}">{}</text>"#,
            ml + 6.0 + (i % 3) as f64 * 96.0,
            mt + 4.0 + (i / 3) as f64 * 12.0,
            PALETTE[s.color % PALETTE.len()],
            ml + 18.0 + (i % 3) as f64 * 96.0,
            mt + 12.0 + (i / 3) as f64 * 12.0,
            esc(&s.name)
        );
    }
    let _ = write!(
        o,
        r#"<text x="{}" y="{}" fill="{AXIS}" text-anchor="middle">{}</text>"#,
        (ml + w - mr) / 2.0,
        h - 4.0,
        esc(c.x_label)
    );
    let _ = write!(
        o,
        r#"<text x="10" y="{}" fill="{AXIS}" text-anchor="middle" transform="rotate(-90 10 {})">{}</text></svg>"#,
        (mt + h - mb) / 2.0,
        (mt + h - mb) / 2.0,
        esc(c.y_label)
    );
    o
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_valid_looking_svg_with_every_series_style() {
        let s = vec![
            Series::new("Exp", vec![(10.0, 1.0), (20.0, 2.5), (30.0, 3.0)], Style::Dots, 0),
            Series::new("Modelo", vec![(0.0, 0.0), (15.0, 2.0), (30.0, 3.2)], Style::Line, 1),
            Series::new("Meseta", vec![(0.0, 3.5), (30.0, 3.5)], Style::Dashed, 2),
            Series::new("Hist <b>", vec![(5.0, 0.2), (15.0, 0.5)], Style::Bars, 3),
        ];
        let svg = render(&Chart {
            title: "Variograma & <ejes>",
            x_label: "h (m)",
            y_label: "γ",
            series: &s,
            width: 320.0,
            height: 200.0,
            y_from_zero: true,
        });
        assert!(svg.starts_with("<svg") && svg.ends_with("</svg>"));
        assert_eq!(svg.matches("<svg").count(), 1);
        assert!(svg.contains("<path") && svg.contains("<circle") && svg.contains("<rect"));
        assert!(svg.contains("stroke-dasharray"));
        assert!(
            svg.contains("Variograma &amp; &lt;ejes&gt;") && svg.contains("Hist &lt;b&gt;"),
            "text must be escaped"
        );
        assert!(!svg.contains("NaN") && !svg.contains("inf"));
    }

    #[test]
    fn empty_and_degenerate_inputs_do_not_break() {
        let empty = render(&Chart {
            title: "t",
            x_label: "",
            y_label: "",
            series: &[],
            width: 100.0,
            height: 80.0,
            y_from_zero: false,
        });
        assert!(empty.ends_with("</svg>"));
        let one = [Series::new("", vec![(5.0, 5.0), (f64::NAN, 1.0)], Style::Dots, 0)];
        let svg = render(&Chart {
            title: "t",
            x_label: "x",
            y_label: "y",
            series: &one,
            width: 100.0,
            height: 80.0,
            y_from_zero: false,
        });
        assert!(!svg.contains("NaN"));
    }

    #[test]
    fn ticks_are_nice() {
        assert_eq!(nice_step(100.0, 5), 20.0);
        assert_eq!(nice_step(0.9, 5), 0.2);
        assert_eq!(fmt_tick(2.50), "2.5");
        assert_eq!(fmt_tick(40.0), "40");
    }
}
