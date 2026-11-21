//! XDATA schema v1 (application `OPENMINING`). See `docs/PLUGIN.md`.
//!
//! Record = list of strings: first the class identity
//! `OPENMINING-CLASS:<KEY>` (read by the host for display labels), then
//! `schema=1`, then `key=value` pairs. Text keeps the record DWG/DXF-safe and
//! readable by any tool; numbers are written with `{}` (shortest round-trip).

pub const APP: &str = "OPENMINING";
/// Second application: marks solids as estimation domains (kept apart so it
/// does not replace the record of the solid itself).
pub const APP_DOMAIN: &str = "OPENMINING_DOM";
pub const CLASS_PREFIX: &str = "OPENMINING-CLASS:";
pub const SCHEMA: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Class {
    Trace,
    Collar,
    Assay,
    Geo,
    Label,
    Solid,
    Section,
    GeoTrace,
    GeoProj,
    Axis,
    Cut,
    Sheet,
    Comp,
    Surface,
    View,
    /// A solid assigned to an estimation domain (`APP_DOMAIN`).
    DomainSolid,
    /// A classified geotechnical record (interval, mapped stretch or point).
    GtRec,
    /// Stereonet or design chart drawn in the DWG (`kind`).
    RmDraw,
    /// A wedge of three discontinuity sets on a working.
    RmWedge,
    /// Hanging-wall or footwall surface of a vein.
    VnSurface,
    /// Prism of a vein model.
    VnPrism,
    /// Long section of a vein model.
    VnLong,
    /// Block label or working drawn on a vein long section.
    VnBlock,
}

impl Class {
    pub const ALL: [Class; 23] = [
        Class::Trace,
        Class::Collar,
        Class::Assay,
        Class::Geo,
        Class::Label,
        Class::Solid,
        Class::Section,
        Class::GeoTrace,
        Class::GeoProj,
        Class::Axis,
        Class::Cut,
        Class::Sheet,
        Class::Comp,
        Class::Surface,
        Class::View,
        Class::DomainSolid,
        Class::GtRec,
        Class::RmDraw,
        Class::RmWedge,
        Class::VnSurface,
        Class::VnPrism,
        Class::VnLong,
        Class::VnBlock,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Self::Trace => "DH_TRACE",
            Self::Collar => "DH_COLLAR",
            Self::Assay => "DH_ASSAY",
            Self::Geo => "DH_GEO",
            Self::Label => "DH_LABEL",
            Self::Solid => "UG_SOLID",
            Self::Section => "UG_SECTION",
            Self::GeoTrace => "GM_TRACE",
            Self::GeoProj => "GM_PROJ",
            Self::Axis => "AX_AXIS",
            Self::Cut => "SC_CUT",
            Self::Sheet => "SC_SHEET",
            Self::Comp => "RS_COMP",
            Self::Surface => "RS_SURFACE",
            Self::View => "RS_VIEW",
            Self::DomainSolid => "RS_DOMAIN",
            Self::GtRec => "GT_REC",
            Self::RmDraw => "RM_DRAW",
            Self::RmWedge => "RM_WEDGE",
            Self::VnSurface => "VN_SURFACE",
            Self::VnPrism => "VN_PRISM",
            Self::VnLong => "VN_LONG",
            Self::VnBlock => "VN_BLOCK",
        }
    }

    pub fn from_key(k: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|c| c.key() == k)
    }

    pub fn is_drillhole(self) -> bool {
        matches!(self, Self::Trace | Self::Collar | Self::Assay | Self::Geo | Self::Label)
    }
}

/// Identity + attributes stored on an entity.
#[derive(Debug, Clone, PartialEq)]
pub struct Tag {
    pub class: Class,
    pub fields: Vec<(String, String)>,
}

impl Tag {
    pub fn new(class: Class) -> Self {
        Tag {
            class,
            fields: Vec::new(),
        }
    }

    pub fn with(mut self, k: &str, v: impl ToString) -> Self {
        self.fields.push((k.to_string(), v.to_string()));
        self
    }

    pub fn get(&self, k: &str) -> Option<&str> {
        self.fields.iter().find(|(key, _)| key == k).map(|(_, v)| v.as_str())
    }

    pub fn hole(&self) -> Option<&str> {
        self.get("hole")
    }

    pub fn encode(&self) -> Vec<String> {
        let mut v = vec![
            format!("{CLASS_PREFIX}{}", self.class.key()),
            format!("schema={SCHEMA}"),
        ];
        v.extend(
            self.fields
                .iter()
                .map(|(k, val)| format!("{k}={}", val.replace(['\r', '\n'], " "))),
        );
        v
    }

    /// Decodes a record; unknown schema versions newer than ours are still
    /// read best-effort (keys are additive).
    pub fn decode<'a>(values: impl IntoIterator<Item = &'a str>) -> Option<Self> {
        let mut it = values.into_iter();
        let class = Class::from_key(it.next()?.strip_prefix(CLASS_PREFIX)?)?;
        let mut tag = Tag::new(class);
        for s in it {
            if let Some((k, v)) = s.split_once('=') {
                if k != "schema" {
                    tag.fields.push((k.to_string(), v.to_string()));
                }
            }
        }
        Some(tag)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let t = Tag::new(Class::Assay)
            .with("hole", "DH-01")
            .with("from", 1.5)
            .with("cu%", 0.25);
        let enc = t.encode();
        assert_eq!(enc[0], "OPENMINING-CLASS:DH_ASSAY");
        assert_eq!(enc[1], "schema=1");
        let back = Tag::decode(enc.iter().map(|s| s.as_str())).unwrap();
        assert_eq!(back, t);
        assert_eq!(back.get("from"), Some("1.5"));
    }

    #[test]
    fn foreign_records_ignored() {
        assert!(Tag::decode(["SOMETHING-ELSE"]).is_none());
    }
}
