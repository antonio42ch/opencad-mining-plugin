//! Rows -> records. Columns are found by alias, or by an explicit
//! [`ColumnMap`] when the user maps them by hand. Missing/invalid numbers
//! become `NaN` so the validator (R05) rejects them explicitly.

use std::collections::{BTreeMap, HashMap};

use crate::error::{ImportError, TableKind};
use crate::model::*;
use crate::table::{normalize_header, parse_number, RawTable};

/// A logical column the importer needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Field {
    HoleId,
    X,
    Y,
    Z,
    MaxDepth,
    Azimuth,
    Dip,
    Depth,
    From,
    To,
}

impl Field {
    pub fn key(self) -> &'static str {
        match self {
            Self::HoleId => "hole_id",
            Self::X => "x",
            Self::Y => "y",
            Self::Z => "z",
            Self::MaxDepth => "max_depth",
            Self::Azimuth => "azimuth",
            Self::Dip => "dip",
            Self::Depth => "depth",
            Self::From => "from",
            Self::To => "to",
        }
    }

    pub fn from_key(k: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|f| f.key() == k)
    }

    pub const ALL: [Field; 10] = [
        Field::HoleId,
        Field::X,
        Field::Y,
        Field::Z,
        Field::MaxDepth,
        Field::Azimuth,
        Field::Dip,
        Field::Depth,
        Field::From,
        Field::To,
    ];

    fn aliases(self) -> &'static [&'static str] {
        match self {
            Self::HoleId => &[
                "holeid",
                "sampleid",
                "sample",
                "id",
                "bhid",
                "drillhole",
                "hid",
                "traceid",
                "dhid",
                "sondaje",
                "pozo",
            ],
            Self::X => &["x", "easting", "este", "locationx", "coordx", "coorde", "east"],
            Self::Y => &["y", "northing", "norte", "locationy", "coordy", "coordn", "north"],
            Self::Z => &[
                "z",
                "elevation",
                "elevacion",
                "elevación",
                "elev",
                "rl",
                "locationz",
                "coordz",
                "cota",
            ],
            Self::MaxDepth => &[
                "maxdepth",
                "depth",
                "profundidad",
                "length",
                "eoh",
                "totaldepth",
                "longitud",
            ],
            Self::Azimuth => &["azimuth", "az", "bearing", "azimut"],
            Self::Dip => &["dip", "inclinacion", "inclinación", "buzamiento"],
            Self::Depth => &["depth", "md", "profundidad", "at", "distance"],
            Self::From => &["from", "desde"],
            Self::To => &["to", "hasta"],
        }
    }

    /// Fields a table needs (required, optional).
    pub fn for_table(table: TableKind) -> (&'static [Field], &'static [Field]) {
        match table {
            TableKind::Collar => (
                &[Field::HoleId, Field::X, Field::Y, Field::Z, Field::MaxDepth],
                &[Field::Azimuth, Field::Dip],
            ),
            TableKind::Survey => (
                &[Field::HoleId, Field::Azimuth, Field::Dip],
                &[Field::Depth, Field::From, Field::To],
            ),
            TableKind::Assay | TableKind::Geo => (&[Field::HoleId, Field::From, Field::To], &[]),
        }
    }
}

/// Manual header choice per field (header text as it appears in the file).
pub type ColumnMap = HashMap<Field, String>;

struct Table<'a> {
    kind: TableKind,
    raw_headers: Vec<String>,
    headers: Vec<String>,
    rows: &'a [Vec<String>],
    decimal_comma: bool,
    map: &'a ColumnMap,
}

impl<'a> Table<'a> {
    fn new(raw: &'a RawTable, kind: TableKind, map: &'a ColumnMap) -> Result<Self, ImportError> {
        let (head, rows) = raw.rows.split_first().ok_or(ImportError::Empty { table: kind })?;
        Ok(Table {
            kind,
            raw_headers: head.iter().map(|h| h.trim().to_string()).collect(),
            headers: head.iter().map(|h| normalize_header(h)).collect(),
            rows,
            decimal_comma: raw.decimal_comma,
            map,
        })
    }

    fn find(&self, f: Field) -> Result<Option<usize>, ImportError> {
        if let Some(name) = self.map.get(&f) {
            let n = normalize_header(name);
            return self.headers.iter().position(|h| *h == n).map(Some).ok_or_else(|| {
                ImportError::UnknownMappedColumn {
                    table: self.kind,
                    field: f.key(),
                    column: name.clone(),
                }
            });
        }
        Ok(self.headers.iter().position(|h| f.aliases().contains(&h.as_str())))
    }

    fn require(&self, f: Field) -> Result<usize, ImportError> {
        self.find(f)?.ok_or_else(|| {
            if f == Field::HoleId {
                ImportError::MissingIdColumn {
                    table: self.kind,
                    headers: self.raw_headers.clone(),
                }
            } else {
                ImportError::MissingColumn {
                    table: self.kind,
                    field: f.key(),
                    headers: self.raw_headers.clone(),
                }
            }
        })
    }

    fn cell<'r>(&self, row: &'r [String], col: usize) -> &'r str {
        row.get(col).map(|s| s.trim()).unwrap_or("")
    }

    fn num(&self, row: &[String], col: usize) -> f64 {
        parse_number(self.cell(row, col), self.decimal_comma).unwrap_or(f64::NAN)
    }

    fn opt(&self, row: &[String], col: Option<usize>) -> Option<f64> {
        col.and_then(|c| parse_number(self.cell(row, c), self.decimal_comma))
    }

    /// Data rows with a non-empty id, paired with their 1-based file row.
    fn records(&self, id: usize) -> impl Iterator<Item = (usize, &'a [String])> + '_ {
        self.rows
            .iter()
            .enumerate()
            .filter(move |(_, r)| !self.cell(r, id).is_empty())
            .map(|(i, r)| (i + 2, r.as_slice()))
    }
}

pub fn parse_collars(raw: &RawTable, map: &ColumnMap) -> Result<Vec<Collar>, ImportError> {
    let t = Table::new(raw, TableKind::Collar, map)?;
    let id = t.require(Field::HoleId)?;
    let (cx, cy, cz, cd) = (
        t.require(Field::X)?,
        t.require(Field::Y)?,
        t.require(Field::Z)?,
        t.require(Field::MaxDepth)?,
    );
    let (caz, cdip) = (t.find(Field::Azimuth)?, t.find(Field::Dip)?);
    Ok(t.records(id)
        .map(|(_, r)| Collar {
            hole_id: t.cell(r, id).to_string(),
            x: t.num(r, cx),
            y: t.num(r, cy),
            z: t.num(r, cz),
            max_depth: t.num(r, cd),
            azimuth: t.opt(r, caz),
            dip: t.opt(r, cdip),
        })
        .collect())
}

/// Two layouts: standard (`Depth, Azimuth, Dip`) or FROM-TO. In FROM-TO each
/// row yields a station at `FROM` and one at `TO` with the same direction.
pub fn parse_surveys(raw: &RawTable, map: &ColumnMap) -> Result<Vec<Survey>, ImportError> {
    let t = Table::new(raw, TableKind::Survey, map)?;
    let id = t.require(Field::HoleId)?;
    let (caz, cdip) = (t.require(Field::Azimuth)?, t.require(Field::Dip)?);
    let mut out = Vec::new();
    if let Some(cd) = t.find(Field::Depth)? {
        for (_, r) in t.records(id) {
            out.push(Survey {
                hole_id: t.cell(r, id).to_string(),
                depth: t.num(r, cd),
                azimuth: t.num(r, caz),
                dip: t.num(r, cdip),
            });
        }
    } else if let (Some(cf), Some(ct)) = (t.find(Field::From)?, t.find(Field::To)?) {
        for (_, r) in t.records(id) {
            let (az, dip) = (t.num(r, caz), t.num(r, cdip));
            for d in [t.num(r, cf), t.num(r, ct)] {
                out.push(Survey {
                    hole_id: t.cell(r, id).to_string(),
                    depth: d,
                    azimuth: az,
                    dip,
                });
            }
        }
    } else {
        return Err(ImportError::MissingColumn {
            table: TableKind::Survey,
            field: Field::Depth.key(),
            headers: t.raw_headers.clone(),
        });
    }
    Ok(out)
}

/// Assay value: empty -> None; `<x` -> x/2 (R04); number -> value; else None.
pub fn parse_assay_value(s: &str, decimal_comma: bool) -> Option<f64> {
    let s = s.trim();
    match s.strip_prefix('<') {
        Some(rest) => parse_number(rest, decimal_comma).map(|v| v / 2.0),
        None => parse_number(s, decimal_comma),
    }
}

fn attribute_columns(t: &Table, fixed: &[usize]) -> Vec<(usize, String)> {
    t.headers
        .iter()
        .enumerate()
        .filter(|(i, h)| !fixed.contains(i) && !h.is_empty())
        .map(|(i, h)| (i, h.clone()))
        .collect()
}

pub fn parse_assays(raw: &RawTable, map: &ColumnMap) -> Result<Vec<Assay>, ImportError> {
    let t = Table::new(raw, TableKind::Assay, map)?;
    let (id, cf, ct) = (
        t.require(Field::HoleId)?,
        t.require(Field::From)?,
        t.require(Field::To)?,
    );
    let attrs = attribute_columns(&t, &[id, cf, ct]);
    Ok(t.records(id)
        .map(|(_, r)| Assay {
            hole_id: t.cell(r, id).to_string(),
            from: t.num(r, cf),
            to: t.num(r, ct),
            attributes: attrs
                .iter()
                .map(|(i, h)| (h.clone(), parse_assay_value(t.cell(r, *i), t.decimal_comma)))
                .collect(),
        })
        .collect())
}

pub fn parse_geo(raw: &RawTable, map: &ColumnMap) -> Result<Vec<GeoLog>, ImportError> {
    let t = Table::new(raw, TableKind::Geo, map)?;
    let (id, cf, ct) = (
        t.require(Field::HoleId)?,
        t.require(Field::From)?,
        t.require(Field::To)?,
    );
    let attrs = attribute_columns(&t, &[id, cf, ct]);
    Ok(t.records(id)
        .map(|(_, r)| GeoLog {
            hole_id: t.cell(r, id).to_string(),
            from: t.num(r, cf),
            to: t.num(r, ct),
            attributes: attrs
                .iter()
                .filter_map(|(i, h)| {
                    let v = t.cell(r, *i);
                    (!v.is_empty()).then(|| (h.clone(), v.to_string()))
                })
                .collect::<BTreeMap<_, _>>(),
        })
        .collect())
}

/// Numeric attribute names present in the assays (sorted).
pub fn assay_attributes(assays: &[Assay]) -> Vec<String> {
    let mut v: Vec<String> = assays.iter().flat_map(|a| a.attributes.keys().cloned()).collect();
    v.sort();
    v.dedup();
    v
}

/// Text attribute names present in the geology logs (sorted).
pub fn geo_attributes(geo: &[GeoLog]) -> Vec<String> {
    let mut v: Vec<String> = geo.iter().flat_map(|g| g.attributes.keys().cloned()).collect();
    v.sort();
    v.dedup();
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    fn csv(s: &str) -> RawTable {
        RawTable::from_csv_bytes(s.as_bytes())
    }

    #[test]
    fn collar_aliases() {
        let c = parse_collars(
            &csv("HOLE-ID,LOCATION X,LOCATION Y,LOCATION Z,LENGTH\nDH1,10,20,100,50\n,1,1,1,1\n"),
            &ColumnMap::new(),
        )
        .unwrap();
        assert_eq!(c.len(), 1);
        assert_eq!((c[0].x, c[0].y, c[0].z, c[0].max_depth), (10.0, 20.0, 100.0, 50.0));
    }

    #[test]
    fn manual_mapping_overrides_aliases() {
        let raw = csv("POZO_ID,E,N,COTA_BOCA,PROF\nDH1,1,2,3,4\n");
        let err = parse_collars(&raw, &ColumnMap::new()).unwrap_err();
        assert_eq!(err.headers().unwrap().len(), 5);
        let map: ColumnMap = [
            (Field::HoleId, "POZO_ID"),
            (Field::X, "E"),
            (Field::Y, "N"),
            (Field::Z, "COTA_BOCA"),
            (Field::MaxDepth, "PROF"),
        ]
        .into_iter()
        .map(|(f, s)| (f, s.to_string()))
        .collect();
        let c = parse_collars(&raw, &map).unwrap();
        assert_eq!((c[0].x, c[0].z, c[0].max_depth), (1.0, 3.0, 4.0));
    }

    #[test]
    fn unknown_mapped_column_is_reported() {
        let map: ColumnMap = [(Field::X, "NOPE".to_string())].into_iter().collect();
        let err = parse_collars(&csv("HOLEID,X,Y,Z,DEPTH\nA,1,1,1,1\n"), &map).unwrap_err();
        assert!(matches!(err, ImportError::UnknownMappedColumn { .. }));
    }

    #[test]
    fn survey_from_to_makes_two_stations() {
        let s = parse_surveys(
            &csv("HOLE-ID,FROM,TO,DIP,AZIMUTH\nDH1,0,30,-60,45\n"),
            &ColumnMap::new(),
        )
        .unwrap();
        assert_eq!(s.iter().map(|x| x.depth).collect::<Vec<_>>(), vec![0.0, 30.0]);
    }

    #[test]
    fn assay_values_with_decimal_comma() {
        let a = parse_assays(
            &csv("HOLEID;FROM;TO;AU;CU%\nDH1;0;1;<0,05;1,5\nDH1;1;2;;abc\n"),
            &ColumnMap::new(),
        )
        .unwrap();
        assert_eq!(a[0].attributes["au"], Some(0.025));
        assert_eq!(a[0].attributes["cu%"], Some(1.5));
        assert_eq!(a[1].attributes["au"], None);
        assert_eq!(assay_attributes(&a), vec!["au", "cu%"]);
    }
}
