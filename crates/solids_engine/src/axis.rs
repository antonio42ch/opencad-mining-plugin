//! Solid along a 3D axis with a constant section.

use mining_core::{add, scale, Vec2, Vec3};

use crate::frame::{aaa_frame, tangents};
use crate::polyline::{clean_vertices, sample_by_distance, total_length};
use crate::section::Profile;
use crate::{Ring, SolidError};

pub struct AxisParams<'a> {
    pub axis: &'a [Vec3],
    pub profile: &'a Profile,
    pub interval: f64,
    /// Added to every profile point (see [`crate::section::alignment_offset`]).
    pub offset: Vec2,
}

pub fn axis_rings(p: &AxisParams) -> Result<Vec<Ring>, SolidError> {
    if p.interval.is_nan() || p.interval <= 0.0 {
        return Err(SolidError::InvalidInterval(p.interval));
    }
    let axis = clean_vertices(p.axis.to_vec());
    if axis.len() < 2 {
        return Err(SolidError::TooFewVertices { what: "axis" });
    }
    let stations = sample_by_distance(&axis, p.interval);
    if stations.len() < 2 {
        return Err(SolidError::IntervalTooLong {
            interval: p.interval,
            length: total_length(&axis),
        });
    }
    let profile = p.profile.points()?;
    Ok(stations
        .iter()
        .zip(tangents(&stations))
        .map(|(s, t)| {
            let (right, up) = aaa_frame(t);
            profile
                .iter()
                .map(|q| add(*s, add(scale(right, q[0] + p.offset[0]), scale(up, q[1] + p.offset[1]))))
                .collect()
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mesh::ClosedMesh;

    #[test]
    fn straight_rectangular_gallery_volume() {
        let axis = [[0.0, 0.0, 0.0], [0.0, 50.0, 0.0]];
        let prof = Profile::Rectangular {
            width: 4.0,
            height: 3.0,
        };
        let rings = axis_rings(&AxisParams {
            axis: &axis,
            profile: &prof,
            interval: 5.0,
            offset: [0.0, 0.0],
        })
        .unwrap();
        assert_eq!(rings.len(), 11);
        let m = ClosedMesh::from_rings(&rings).unwrap();
        assert!((m.volume() - 4.0 * 3.0 * 50.0).abs() < 1e-9);
    }

    #[test]
    fn errors() {
        let prof = Profile::Rectangular {
            width: 4.0,
            height: 3.0,
        };
        let axis = [[0.0; 3], [0.0, 1.0, 0.0]];
        assert!(matches!(
            axis_rings(&AxisParams {
                axis: &axis,
                profile: &prof,
                interval: 0.0,
                offset: [0.0; 2]
            }),
            Err(SolidError::InvalidInterval(_))
        ));
        assert!(matches!(
            axis_rings(&AxisParams {
                axis: &axis[..1],
                profile: &prof,
                interval: 1.0,
                offset: [0.0; 2]
            }),
            Err(SolidError::TooFewVertices { .. })
        ));
    }
}
