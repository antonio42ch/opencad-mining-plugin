//! The block model: geometry + domain per block + named f32 columns.

use std::collections::BTreeMap;

use crate::geometry::BlockGeometry;

#[derive(Debug, Clone, PartialEq)]
pub struct BlockModel {
    pub geometry: BlockGeometry,
    /// Domain names; the block domain is a 1-based index, 0 = outside.
    pub domain_names: Vec<String>,
    pub domain: Vec<u8>,
    /// Fraction of the block inside its domain (0..1).
    pub fraction: Vec<f32>,
    pub columns: BTreeMap<String, Vec<f32>>,
}

impl BlockModel {
    pub fn new(geometry: BlockGeometry) -> Self {
        let n = geometry.len();
        BlockModel {
            geometry,
            domain_names: Vec::new(),
            domain: vec![0; n],
            fraction: vec![0.0; n],
            columns: BTreeMap::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.domain.len()
    }

    pub fn is_empty(&self) -> bool {
        self.domain.is_empty()
    }

    /// Column by name, creating it filled with NaN when missing.
    pub fn column_mut(&mut self, name: &str) -> &mut Vec<f32> {
        let n = self.len();
        self.columns
            .entry(name.to_string())
            .or_insert_with(|| vec![f32::NAN; n])
    }

    pub fn column(&self, name: &str) -> Option<&[f32]> {
        self.columns.get(name).map(|v| v.as_slice())
    }

    pub fn domain_name(&self, id: u8) -> &str {
        id.checked_sub(1)
            .and_then(|i| self.domain_names.get(i as usize))
            .map_or("", |s| s.as_str())
    }

    /// Volume of block `idx` inside its domain.
    pub fn inside_volume(&self, idx: usize) -> f64 {
        self.geometry.volume() * self.fraction[idx] as f64
    }

    /// Number of blocks with a domain.
    pub fn assigned(&self) -> usize {
        self.domain.iter().filter(|&&d| d != 0).count()
    }
}
