//! Geotechnical tables: drillhole logging, face/cell mapping, laboratory
//! tests and oriented discontinuities. Columns are detected from the header
//! (es/en/pt aliases) and can be overridden per field.

use std::collections::BTreeMap;

use drillhole_engine::table::{normalize_header, parse_number, RawTable};

/// Every column the geotechnical module understands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GtField {
    Hole,
    From,
    To,
    X,
    Y,
    Z,
    Axis,
    Chainage,
    Lith,
    Rqd,
    Ff,
    Spacing,
    Ucs,
    Is50,
    Persistence,
    Aperture,
    Roughness,
    Infilling,
    Weathering,
    Water,
    RStrength,
    RRqd,
    RSpacing,
    RCondition,
    RWater,
    Rmr,
    Jn,
    Jr,
    Ja,
    Jw,
    Srf,
    Q,
    Gsi,
    Dip,
    DipDir,
    Mi,
    Ei,
    Poisson,
    Density,
}

use GtField::*;

impl GtField {
    pub const ALL: [GtField; 39] = [
        Hole,
        From,
        To,
        X,
        Y,
        Z,
        Axis,
        Chainage,
        Lith,
        Rqd,
        Ff,
        Spacing,
        Ucs,
        Is50,
        Persistence,
        Aperture,
        Roughness,
        Infilling,
        Weathering,
        Water,
        RStrength,
        RRqd,
        RSpacing,
        RCondition,
        RWater,
        Rmr,
        Jn,
        Jr,
        Ja,
        Jw,
        Srf,
        Q,
        Gsi,
        Dip,
        DipDir,
        Mi,
        Ei,
        Poisson,
        Density,
    ];

    /// Stable key (configuration, XDATA, messages).
    pub fn key(self) -> &'static str {
        match self {
            Hole => "hole",
            From => "from",
            To => "to",
            X => "x",
            Y => "y",
            Z => "z",
            Axis => "axis",
            Chainage => "chainage",
            Lith => "lith",
            Rqd => "rqd",
            Ff => "ff",
            Spacing => "spacing",
            Ucs => "ucs",
            Is50 => "is50",
            Persistence => "persistence",
            Aperture => "aperture",
            Roughness => "roughness",
            Infilling => "infilling",
            Weathering => "weathering",
            Water => "water",
            RStrength => "r_strength",
            RRqd => "r_rqd",
            RSpacing => "r_spacing",
            RCondition => "r_condition",
            RWater => "r_water",
            Rmr => "rmr",
            Jn => "jn",
            Jr => "jr",
            Ja => "ja",
            Jw => "jw",
            Srf => "srf",
            Q => "q",
            Gsi => "gsi",
            Dip => "dip",
            DipDir => "dipdir",
            Mi => "mi",
            Ei => "ei",
            Poisson => "poisson",
            Density => "density",
        }
    }

    pub fn from_key(k: &str) -> Option<GtField> {
        Self::ALL.into_iter().find(|f| f.key() == k)
    }

    /// Header aliases, already normalised (lowercase, no `-_ .`).
    fn aliases(self) -> &'static [&'static str] {
        match self {
            Hole => &[
                "holeid", "hole", "bhid", "sondaje", "sondeo", "dhid", "furo", "pozo", "taladro",
            ],
            From => &["from", "desde", "de", "inicio", "fromm"],
            To => &["to", "hasta", "a", "ate", "fin", "tom"],
            X => &["x", "este", "east", "easting", "e", "xcoord"],
            Y => &["y", "norte", "north", "northing", "n", "ycoord"],
            Z => &[
                "z",
                "cota",
                "elevacion",
                "elevación",
                "elevation",
                "rl",
                "zcoord",
                "altura",
            ],
            Axis => &["axis", "eje", "labor", "galeria", "galería", "tunnel", "tunel", "túnel"],
            Chainage => &["chainage", "progresiva", "pk", "estaca", "station", "abscisa"],
            Lith => &[
                "lith",
                "litologia",
                "litología",
                "lithology",
                "roca",
                "rock",
                "dominio",
                "domain",
                "unidad",
                "litho",
            ],
            Rqd => &["rqd", "rqd%"],
            Ff => &[
                "ff",
                "frecuenciafracturas",
                "fracturasm",
                "fracturaspormetro",
                "ffm",
                "fracturefrequency",
                "fpm",
            ],
            Spacing => &["spacing", "espaciado", "espaciamiento", "spacingm", "espaciamientom"],
            Ucs => &[
                "ucs",
                "rcu",
                "ucsmpa",
                "rcs",
                "ucs(mpa)",
                "resistencia",
                "sigmaci",
                "sci",
            ],
            Is50 => &["is50", "is(50)", "plt", "cargapuntual", "pointload"],
            Persistence => &["persistence", "persistencia", "continuidad", "longitud"],
            Aperture => &[
                "aperture",
                "apertura",
                "abertura",
                "separation",
                "separacion",
                "separación",
            ],
            Roughness => &["roughness", "rugosidad"],
            Infilling => &["infilling", "relleno", "filling", "preenchimento"],
            Weathering => &[
                "weathering",
                "meteorizacion",
                "meteorización",
                "alteracion",
                "alteración",
                "intemperismo",
            ],
            Water => &["water", "agua", "groundwater", "aguasubterranea", "condicionagua"],
            RStrength => &["rstrength", "ruc", "r1", "rmrucs", "puntajeucs"],
            RRqd => &["rrqd", "r2", "rmrrqd", "puntajerqd"],
            RSpacing => &["rspacing", "r3", "rmrspacing", "puntajeespaciado"],
            RCondition => &[
                "rcondition",
                "r4",
                "rmrcondition",
                "condicion",
                "condición",
                "condiciondiscontinuidades",
                "jc",
            ],
            RWater => &["rwater", "r5", "rmrwater", "puntajeagua"],
            Rmr => &["rmr", "rmr89", "rmrbasico", "rmrb"],
            Jn => &["jn"],
            Jr => &["jr"],
            Ja => &["ja"],
            Jw => &["jw"],
            Srf => &["srf"],
            Q => &["q", "qbarton", "qindex"],
            Gsi => &["gsi"],
            Dip => &["dip", "buzamiento", "bz", "mergulho"],
            DipDir => &[
                "dipdir",
                "dipdirection",
                "dirbuzamiento",
                "direccionbuzamiento",
                "direcciondebuzamiento",
                "dd",
                "azbuz",
            ],
            Mi => &["mi"],
            Ei => &["ei", "e", "modulo", "módulo", "young", "ei(mpa)", "emod"],
            Poisson => &["poisson", "nu", "v", "coefpoisson"],
            Density => &["density", "densidad", "pe", "pesoespecifico", "gamma", "sg"],
        }
    }
}

/// What a table is used for: decides the required columns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GtKind {
    /// Drillhole logging by interval.
    Logging,
    /// Face / cell mapping along the workings.
    Mapping,
    /// Laboratory tests by lithology.
    Lab,
    /// Oriented discontinuities (stored now, analysed in phase 3).
    Joints,
}

impl GtKind {
    pub fn key(self) -> &'static str {
        match self {
            GtKind::Logging => "logging",
            GtKind::Mapping => "mapping",
            GtKind::Lab => "lab",
            GtKind::Joints => "joints",
        }
    }

    /// Columns that are searched for this kind of table (keeps `e` = east
    /// from being read as a modulus in a mapping table, etc.).
    fn fields(self) -> Vec<GtField> {
        let rating: &[GtField] = &[
            Lith,
            Rqd,
            Ff,
            Spacing,
            Ucs,
            Is50,
            Persistence,
            Aperture,
            Roughness,
            Infilling,
            Weathering,
            Water,
            RStrength,
            RRqd,
            RSpacing,
            RCondition,
            RWater,
            Rmr,
            Jn,
            Jr,
            Ja,
            Jw,
            Srf,
            Q,
            Gsi,
        ];
        let mut v = match self {
            GtKind::Logging => vec![Hole, From, To],
            GtKind::Mapping => vec![X, Y, Z, Axis, Chainage, From, To],
            GtKind::Lab => return vec![Lith, Ucs, Is50, Mi, Ei, Poisson, Density],
            GtKind::Joints => {
                return vec![
                    Hole,
                    From,
                    X,
                    Y,
                    Z,
                    Lith,
                    Dip,
                    DipDir,
                    Persistence,
                    Aperture,
                    Roughness,
                    Infilling,
                ]
            }
        };
        v.extend_from_slice(rating);
        v
    }

    /// Missing required columns, given the mapped ones.
    fn missing(self, have: &BTreeMap<GtField, usize>) -> Vec<GtField> {
        let need: Vec<GtField> = match self {
            GtKind::Logging => vec![Hole, From, To],
            GtKind::Lab => vec![Lith],
            GtKind::Joints => vec![Dip, DipDir],
            GtKind::Mapping => {
                if have.contains_key(&Chainage) || (have.contains_key(&X) && have.contains_key(&Y)) {
                    vec![]
                } else {
                    vec![X, Y]
                }
            }
        };
        need.into_iter().filter(|f| !have.contains_key(f)).collect()
    }
}

/// One row: raw cell text per mapped field.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct GtRecord {
    /// 1-based data row (header excluded), for messages.
    pub row: usize,
    pub values: BTreeMap<GtField, String>,
    pub decimal_comma: bool,
}

impl GtRecord {
    pub fn text(&self, f: GtField) -> Option<&str> {
        self.values.get(&f).map(|s| s.trim()).filter(|s| !s.is_empty())
    }
    pub fn num(&self, f: GtField) -> Option<f64> {
        self.text(f).and_then(|s| parse_number(s, self.decimal_comma))
    }
    /// Builder for tests and the CLI.
    pub fn with(mut self, f: GtField, v: impl ToString) -> Self {
        self.values.insert(f, v.to_string());
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GtTable {
    pub kind: GtKind,
    pub records: Vec<GtRecord>,
    /// Field -> header actually used.
    pub mapping: Vec<(GtField, String)>,
    /// Headers that matched no field.
    pub unmapped: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GtError {
    Empty,
    MissingColumns {
        missing: Vec<GtField>,
        headers: Vec<String>,
    },
    UnknownHeader {
        field: GtField,
        header: String,
    },
}

impl std::fmt::Display for GtError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GtError::Empty => write!(f, "the table has no data rows"),
            GtError::MissingColumns { missing, headers } => {
                let m: Vec<&str> = missing.iter().map(|f| f.key()).collect();
                write!(
                    f,
                    "missing column(s) {} (headers: {})",
                    m.join(", "),
                    headers.join(", ")
                )
            }
            GtError::UnknownHeader { field, header } => write!(f, "column '{header}' for {} not found", field.key()),
        }
    }
}
impl std::error::Error for GtError {}

/// Parses a raw table. `overrides` maps a field key to a header name and
/// wins over the detected aliases.
pub fn parse_table(raw: &RawTable, kind: GtKind, overrides: &BTreeMap<String, String>) -> Result<GtTable, GtError> {
    let Some((header, rows)) = raw.rows.split_first() else {
        return Err(GtError::Empty);
    };
    let norm: Vec<String> = header.iter().map(|h| normalize_header(h)).collect();
    let mut cols: BTreeMap<GtField, usize> = BTreeMap::new();
    let mut taken = vec![false; header.len()];
    for (k, h) in overrides {
        let (Some(f), false) = (GtField::from_key(k), h.trim().is_empty()) else {
            continue;
        };
        let hn = normalize_header(h);
        let i = norm.iter().position(|n| *n == hn).ok_or(GtError::UnknownHeader {
            field: f,
            header: h.clone(),
        })?;
        cols.insert(f, i);
        taken[i] = true;
    }
    for f in kind.fields() {
        if cols.contains_key(&f) {
            continue;
        }
        if let Some(i) = (0..norm.len()).find(|&i| !taken[i] && f.aliases().contains(&norm[i].as_str())) {
            cols.insert(f, i);
            taken[i] = true;
        }
    }
    let missing = kind.missing(&cols);
    if !missing.is_empty() {
        return Err(GtError::MissingColumns {
            missing,
            headers: header.clone(),
        });
    }
    if rows.is_empty() {
        return Err(GtError::Empty);
    }
    let records = rows
        .iter()
        .enumerate()
        .map(|(r, row)| GtRecord {
            row: r + 1,
            values: cols
                .iter()
                .map(|(&f, &i)| (f, row.get(i).cloned().unwrap_or_default()))
                .collect(),
            decimal_comma: raw.decimal_comma,
        })
        .collect();
    Ok(GtTable {
        kind,
        records,
        mapping: cols.iter().map(|(&f, &i)| (f, header[i].clone())).collect(),
        unmapped: (0..header.len())
            .filter(|&i| !taken[i])
            .map(|i| header[i].clone())
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw(text: &str) -> RawTable {
        RawTable::from_csv_bytes(text.as_bytes())
    }

    #[test]
    fn logging_aliases_in_spanish_excel() {
        let t = parse_table(
            &raw("SONDAJE;DESDE;HASTA;RQD %;UCS (MPa);Jn;Jr;Ja;Otra\nDH1;0;3,5;85;120,5;9;1,5;1;x\n"),
            GtKind::Logging,
            &BTreeMap::new(),
        )
        .unwrap();
        let r = &t.records[0];
        assert_eq!(r.text(Hole), Some("DH1"));
        assert_eq!(r.num(To), Some(3.5));
        assert_eq!(r.num(Ucs), Some(120.5));
        assert_eq!(r.num(Jr), Some(1.5));
        assert_eq!(t.unmapped, vec!["Otra".to_string()]);
    }

    #[test]
    fn required_columns_and_overrides() {
        let e = parse_table(&raw("id,rqd\nA,50\n"), GtKind::Logging, &BTreeMap::new()).unwrap_err();
        assert!(matches!(e, GtError::MissingColumns { ref missing, .. } if missing.contains(&Hole)));
        let ov: BTreeMap<String, String> = [
            ("hole".to_string(), "id".to_string()),
            ("from".into(), "p1".into()),
            ("to".into(), "p2".into()),
        ]
        .into();
        let t = parse_table(&raw("id,p1,p2,rqd\nA,0,2,50\n"), GtKind::Logging, &ov).unwrap();
        assert_eq!(t.records[0].num(Rqd), Some(50.0));
        let bad: BTreeMap<String, String> = [("hole".to_string(), "nope".to_string())].into();
        assert!(matches!(
            parse_table(&raw("id\nA\n"), GtKind::Logging, &bad),
            Err(GtError::UnknownHeader { .. })
        ));
    }

    #[test]
    fn mapping_needs_coordinates_or_chainage() {
        assert!(parse_table(&raw("pk,rmr\n10,55\n"), GtKind::Mapping, &BTreeMap::new()).is_ok());
        assert!(parse_table(&raw("este,norte,rmr\n1,2,55\n"), GtKind::Mapping, &BTreeMap::new()).is_ok());
        assert!(parse_table(&raw("rmr\n55\n"), GtKind::Mapping, &BTreeMap::new()).is_err());
        // In a mapping table "e" is east, never the modulus.
        let t = parse_table(&raw("e,n,z\n1,2,3\n"), GtKind::Mapping, &BTreeMap::new()).unwrap();
        assert_eq!(t.records[0].num(X), Some(1.0));
    }

    #[test]
    fn lab_table() {
        let t = parse_table(
            &raw("Litologia,UCS,mi,Ei,Poisson\nAndesita,150,25,60000,0.25\n"),
            GtKind::Lab,
            &BTreeMap::new(),
        )
        .unwrap();
        assert_eq!(t.records[0].num(Mi), Some(25.0));
        assert_eq!(t.records[0].num(Ei), Some(60000.0));
    }
}
