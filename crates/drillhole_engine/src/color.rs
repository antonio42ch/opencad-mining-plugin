//! Grade / category colour schemes (ACI indices, converted by the host layer).

use std::collections::BTreeMap;

pub const GRAY: u8 = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorScheme {
    BlueToRed,
    GreenToRed,
    Rainbow,
    Custom,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CustomRange {
    pub min: f64,
    pub max: f64,
    pub aci: u8,
}

pub fn normalized(v: f64, min: f64, max: f64) -> f64 {
    if max <= min {
        0.0
    } else {
        ((v - min) / (max - min)).clamp(0.0, 1.0)
    }
}

pub fn grade_color(scheme: ColorScheme, v: f64, min: f64, max: f64, custom: &[CustomRange]) -> u8 {
    let t = normalized(v, min, max);
    match scheme {
        ColorScheme::BlueToRed => {
            if t < 0.33 {
                5
            } else if t < 0.66 {
                3
            } else if t < 0.85 {
                2
            } else {
                1
            }
        }
        ColorScheme::GreenToRed => {
            if t < 0.5 {
                3
            } else if t < 0.75 {
                2
            } else {
                1
            }
        }
        ColorScheme::Rainbow => [5u8, 4, 3, 2, 30, 1][((t * 5.0) as usize).min(5)],
        ColorScheme::Custom => {
            let mut r: Vec<&CustomRange> = custom.iter().collect();
            r.sort_by(|a, b| a.min.total_cmp(&b.min));
            r.into_iter().find(|c| v >= c.min && v <= c.max).map_or(GRAY, |c| c.aci)
        }
    }
}

/// Spanish colour name -> ACI. Unknown -> gray.
pub fn color_name_to_aci(name: &str) -> u8 {
    match name.trim().to_lowercase().as_str() {
        "rojo" => 1,
        "amarillo" => 2,
        "verde" => 3,
        "cian" => 4,
        "azul" => 5,
        "magenta" => 6,
        "blanco" => 7,
        "gris" => 8,
        "naranja" => 30,
        "verde lima" => 62,
        "azul cielo" => 141,
        "rosa" => 210,
        _ => GRAY,
    }
}

/// Category colour (geology): case-insensitive value -> ACI.
pub fn category_color(map: &BTreeMap<String, String>, value: &str) -> u8 {
    let v = value.to_lowercase();
    map.iter()
        .find(|(k, _)| k.to_lowercase() == v)
        .map_or(GRAY, |(_, c)| color_name_to_aci(c))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blue_to_red_thresholds() {
        let c = |v| grade_color(ColorScheme::BlueToRed, v, 0.0, 100.0, &[]);
        assert_eq!((c(10.0), c(50.0), c(70.0), c(90.0)), (5, 3, 2, 1));
    }

    #[test]
    fn degenerate_range_is_t0() {
        assert_eq!(grade_color(ColorScheme::Rainbow, 5.0, 5.0, 5.0, &[]), 5);
    }

    #[test]
    fn custom_ranges_and_fallback() {
        let r = [
            CustomRange {
                min: 0.0,
                max: 1.0,
                aci: 3,
            },
            CustomRange {
                min: 1.0,
                max: 2.0,
                aci: 1,
            },
        ];
        assert_eq!(grade_color(ColorScheme::Custom, 0.5, 0.0, 0.0, &r), 3);
        assert_eq!(grade_color(ColorScheme::Custom, 9.0, 0.0, 0.0, &r), GRAY);
    }
}
