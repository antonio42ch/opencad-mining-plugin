//! 3D position of geotechnical records: drillhole intervals on the
//! desurveyed trace, face mapping by coordinates or by chainage along an
//! axis (horizontal chainage, as Labores and Secciones measure it).

use drillhole_engine::mapper::interpolate_xyz;
use drillhole_engine::model::hole_key;
use drillhole_engine::pipeline::HoleModel;
use mining_core::{lerp, Vec3};

use crate::data::{GtField, GtRecord};

/// Where a record sits: a segment (interval or mapped stretch) or a point.
#[derive(Debug, Clone, PartialEq)]
pub struct Located {
    pub from: Vec3,
    pub to: Vec3,
    pub centre: Vec3,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocateError {
    UnknownHole(String),
    NoDepth,
    NoCoordinates,
    UnknownAxis(String),
    NoAxis,
    AxisTooShort,
}

/// Point at horizontal chainage `ch` (clamped to the axis).
pub fn point_at_chainage(axis: &[Vec3], ch: f64) -> Option<Vec3> {
    if axis.len() < 2 {
        return None;
    }
    let mut acc = 0.0;
    for w in axis.windows(2) {
        let seg = (w[1][0] - w[0][0]).hypot(w[1][1] - w[0][1]);
        if seg > 1e-12 && ch <= acc + seg {
            return Some(lerp(w[0], w[1], ((ch - acc) / seg).clamp(0.0, 1.0)));
        }
        acc += seg;
    }
    (acc > 1e-12).then(|| *axis.last().unwrap())
}

/// Drillhole interval on its hole.
pub fn locate_interval(rec: &GtRecord, holes: &[HoleModel]) -> Result<Located, LocateError> {
    let id = rec.text(GtField::Hole).unwrap_or("");
    let h = holes
        .iter()
        .find(|h| hole_key(&h.collar.hole_id) == hole_key(id))
        .ok_or_else(|| LocateError::UnknownHole(id.to_string()))?;
    let (Some(f), Some(t)) = (rec.num(GtField::From), rec.num(GtField::To)) else {
        return Err(LocateError::NoDepth);
    };
    if h.trace.is_empty() {
        return Err(LocateError::UnknownHole(id.to_string()));
    }
    Ok(Located {
        from: interpolate_xyz(&h.trace, f),
        to: interpolate_xyz(&h.trace, t),
        centre: interpolate_xyz(&h.trace, (f + t) / 2.0),
    })
}

/// Face / cell mapping record. Coordinates win; otherwise chainage (or
/// From/To chainages for a stretch) along the named axis — the only axis
/// when there is one.
pub fn locate_mapping(rec: &GtRecord, axes: &[(String, Vec<Vec3>)]) -> Result<Located, LocateError> {
    if let (Some(x), Some(y)) = (rec.num(GtField::X), rec.num(GtField::Y)) {
        let p = [x, y, rec.num(GtField::Z).unwrap_or(0.0)];
        return Ok(Located {
            from: p,
            to: p,
            centre: p,
        });
    }
    let (a, b) = match (rec.num(GtField::From), rec.num(GtField::To), rec.num(GtField::Chainage)) {
        (Some(f), Some(t), _) => (f, t),
        (_, _, Some(c)) => (c, c),
        _ => return Err(LocateError::NoCoordinates),
    };
    let axis = match rec.text(GtField::Axis) {
        Some(name) => {
            &axes
                .iter()
                .find(|(n, _)| n.trim().eq_ignore_ascii_case(name))
                .ok_or_else(|| LocateError::UnknownAxis(name.to_string()))?
                .1
        }
        None if axes.len() == 1 => &axes[0].1,
        None => return Err(LocateError::NoAxis),
    };
    let at = |c: f64| point_at_chainage(axis, c).ok_or(LocateError::AxisTooShort);
    Ok(Located {
        from: at(a)?,
        to: at(b)?,
        centre: at((a + b) / 2.0)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use drillhole_engine::model::{Collar, DataSet};

    #[test]
    fn chainage_is_horizontal() {
        let axis = vec![
            [0.0, 0.0, 0.0],
            [30.0, 40.0, 10.0],
            [30.0, 40.0, 20.0],
            [30.0, 100.0, 20.0],
        ];
        assert_eq!(point_at_chainage(&axis, 25.0), Some([15.0, 20.0, 5.0]));
        // The vertical step adds no chainage.
        assert_eq!(point_at_chainage(&axis, 60.0), Some([30.0, 50.0, 20.0]));
        assert_eq!(point_at_chainage(&axis, 999.0), Some([30.0, 100.0, 20.0]));
        assert_eq!(point_at_chainage(&axis[..1], 1.0), None);
    }

    #[test]
    fn mapping_by_coordinates_or_axis() {
        let axes = vec![
            ("GAL-1".to_string(), vec![[0.0, 0.0, 100.0], [100.0, 0.0, 100.0]]),
            ("GAL-2".to_string(), vec![[0.0, 0.0, 0.0], [0.0, 10.0, 0.0]]),
        ];
        let r = GtRecord::default().with(GtField::X, 5).with(GtField::Y, 6);
        assert_eq!(locate_mapping(&r, &axes).unwrap().centre, [5.0, 6.0, 0.0]);
        let r = GtRecord::default()
            .with(GtField::Axis, "gal-1")
            .with(GtField::From, 10)
            .with(GtField::To, 30);
        let l = locate_mapping(&r, &axes).unwrap();
        assert_eq!(
            (l.from, l.to, l.centre),
            ([10.0, 0.0, 100.0], [30.0, 0.0, 100.0], [20.0, 0.0, 100.0])
        );
        assert_eq!(
            locate_mapping(&GtRecord::default().with(GtField::Chainage, 5), &axes),
            Err(LocateError::NoAxis)
        );
        assert_eq!(
            locate_mapping(&GtRecord::default().with(GtField::Chainage, 5), &axes[1..])
                .unwrap()
                .centre,
            [0.0, 5.0, 0.0]
        );
        assert!(matches!(
            locate_mapping(&r.clone().with(GtField::Axis, "X"), &axes),
            Err(LocateError::UnknownAxis(_))
        ));
    }

    #[test]
    fn interval_on_a_vertical_hole() {
        let ds = DataSet {
            collars: vec![Collar {
                hole_id: "DH1".into(),
                x: 10.0,
                y: 20.0,
                z: 100.0,
                max_depth: 50.0,
                azimuth: Some(0.0),
                dip: Some(-90.0),
            }],
            ..Default::default()
        };
        let holes = drillhole_engine::pipeline::build_holes(&ds);
        let r = GtRecord::default()
            .with(GtField::Hole, "dh1")
            .with(GtField::From, 10)
            .with(GtField::To, 20);
        let l = locate_interval(&r, &holes).unwrap();
        assert!(
            (l.centre[2] - 85.0).abs() < 1e-6 && (l.centre[0] - 10.0).abs() < 1e-6,
            "{:?}",
            l.centre
        );
        assert!(matches!(
            locate_interval(&r.clone().with(GtField::Hole, "X"), &holes),
            Err(LocateError::UnknownHole(_))
        ));
    }
}
