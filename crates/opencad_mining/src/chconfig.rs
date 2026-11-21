//! Settings of the channel-sample conversion (part of the project settings).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Orientation of one channel set on screen or typed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct OrientCfg {
    pub channel: String,
    pub azimuth: f64,
    pub inclination: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ChannelsConfig {
    /// Topographic points table and channel sampling sheet.
    pub points: String,
    pub sheet: String,
    /// Prefix of the hole ids.
    pub prefix: String,
    /// 0 centred on the located position, 1 starting there.
    pub anchor: usize,
    /// Added to the point elevation when the sheet has no height (m).
    pub height: f64,
    /// Channel azimuth relative to the working (90 = perpendicular).
    pub azimuth_offset: f64,
    /// Default inclination (positive up).
    pub inclination: f64,
    /// Convert the channels again whenever the drillholes are loaded.
    pub auto: bool,
    /// Write the converted tables next to the drawing.
    pub write_files: bool,
    /// Columns chosen for the sheet: field -> header.
    pub column_map: BTreeMap<String, String>,
    pub orient: Vec<OrientCfg>,
    /// Channel whose orientation `ch_orient` sets.
    pub target: String,
}

impl Default for ChannelsConfig {
    fn default() -> Self {
        ChannelsConfig {
            points: String::new(),
            sheet: String::new(),
            prefix: "CH-".into(),
            anchor: 0,
            height: 0.0,
            azimuth_offset: 90.0,
            inclination: 0.0,
            auto: true,
            write_files: true,
            column_map: BTreeMap::new(),
            orient: Vec::new(),
            target: String::new(),
        }
    }
}
