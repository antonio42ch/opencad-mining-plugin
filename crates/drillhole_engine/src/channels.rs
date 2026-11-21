//! Channel samples taken in underground workings, converted into drillhole
//! data (collar, survey, assays, geology) so every drillhole tool works on
//! them.
//!
//! A channel is located by a topographic point and a distance measured from
//! it along the working (towards another point, or along an azimuth). By
//! default it is horizontal and perpendicular to the direction of the working
//! (across the vein when the working follows it), centred on that position;
//! each channel can carry its own azimuth / inclination.
//!
//! Conventions: azimuth clockwise from north; inclination positive upwards
//! (the drillhole `dip`, negative down, is the same number).

use std::collections::{BTreeMap, HashMap};

use crate::model::{Assay, Collar, GeoLog, Survey};
use crate::table::{normalize_header, parse_number, RawTable};

/// Columns of the channel sheet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ChField {
    Channel,
    Point,
    Distance,
    Toward,
    Azimuth,
    Position,
    Height,
    From,
    To,
    Width,
    Thickness,
    SampleType,
    Working,
    Sample,
    ChannelAzimuth,
    ChannelInclination,
}

use ChField::*;

impl ChField {
    pub const ALL: [ChField; 16] = [
        Channel,
        Point,
        Distance,
        Toward,
        Azimuth,
        Position,
        Height,
        From,
        To,
        Width,
        Thickness,
        SampleType,
        Working,
        Sample,
        ChannelAzimuth,
        ChannelInclination,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Channel => "channel",
            Point => "point",
            Distance => "distance",
            Toward => "toward",
            Azimuth => "azimuth",
            Position => "position",
            Height => "height",
            From => "from",
            To => "to",
            Width => "width",
            Thickness => "thickness",
            SampleType => "type",
            Working => "working",
            Sample => "sample",
            ChannelAzimuth => "channel_azimuth",
            ChannelInclination => "channel_inclination",
        }
    }

    pub fn from_key(k: &str) -> Option<ChField> {
        Self::ALL.into_iter().find(|f| f.key() == k)
    }

    /// Header aliases (normalised: lowercase, no `-_ .` or spaces).
    fn aliases(self) -> &'static [&'static str] {
        match self {
            Channel => &[
                "canal",
                "idcanal",
                "canalid",
                "nrocanal",
                "nºcanal",
                "n°canal",
                "channel",
                "channelid",
                "chid",
            ],
            Point => &[
                "punto",
                "pto",
                "ptotopografico",
                "puntotopografico",
                "estacion",
                "estación",
                "ref",
                "referencia",
                "station",
                "point",
                "ponto",
            ],
            Distance => &["distancia", "dist", "distanciam", "d", "distance", "distancia(m)"],
            Toward => &[
                "hacia",
                "haciapunto",
                "siguiente",
                "ptosiguiente",
                "punto2",
                "pto2",
                "toward",
                "to_point",
                "next",
                "nextpoint",
            ],
            Azimuth => &[
                "azimut",
                "azimuth",
                "azlabor",
                "azavance",
                "rumbolabor",
                "direccion",
                "dirección",
                "direction",
            ],
            Position => &[
                "posicion",
                "posición",
                "pos",
                "ubicacion",
                "ubicación",
                "position",
                "lugar",
            ],
            Height => &["altura", "alt", "h", "height", "alturam"],
            From => &["desde", "de", "from", "inicio"],
            To => &["hasta", "a", "to", "fin", "ate"],
            Width => &[
                "ancho",
                "anchomuestra",
                "anchodemuestra",
                "longitud",
                "long",
                "largo",
                "width",
                "samplewidth",
                "length",
                "largura",
            ],
            Thickness => &[
                "potencia",
                "pot",
                "anchoveta",
                "anchodeveta",
                "potenciaveta",
                "thickness",
                "veinwidth",
                "truewidth",
                "espessura",
                "possança",
            ],
            SampleType => &[
                "tipo",
                "tipomuestra",
                "tipodemuestra",
                "type",
                "sampletype",
                "estructura",
                "estructura",
            ],
            Working => &[
                "labor", "galeria", "galería", "nivel", "tajo", "tajeo", "working", "drift", "level",
            ],
            Sample => &[
                "muestra",
                "codigo",
                "código",
                "codmuestra",
                "idmuestra",
                "sample",
                "sampleid",
                "amostra",
            ],
            ChannelAzimuth => &[
                "azcanal",
                "azimutcanal",
                "azimuthcanal",
                "channelazimuth",
                "azimuthchannel",
            ],
            ChannelInclination => &[
                "inccanal",
                "inclinacioncanal",
                "inclinacióncanal",
                "inclinacion",
                "inclinación",
                "channelinclination",
                "inclination",
                "buzamientocanal",
            ],
        }
    }
}

/// A topographic point.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TopoPoint {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ChError {
    Empty(&'static str),
    MissingColumns {
        table: &'static str,
        missing: Vec<&'static str>,
        headers: Vec<String>,
    },
    UnknownHeader {
        field: String,
        header: String,
    },
}

impl std::fmt::Display for ChError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChError::Empty(t) => write!(f, "the {t} table has no data rows"),
            ChError::MissingColumns {
                table,
                missing,
                headers,
            } => {
                write!(
                    f,
                    "{table}: missing column(s) {} (headers: {})",
                    missing.join(", "),
                    headers.join(", ")
                )
            }
            ChError::UnknownHeader { field, header } => write!(f, "column '{header}' for {field} not found"),
        }
    }
}
impl std::error::Error for ChError {}

fn find(norm: &[String], aliases: &[&str], taken: &[bool]) -> Option<usize> {
    (0..norm.len()).find(|&i| !taken[i] && aliases.contains(&norm[i].as_str()))
}

/// Topographic points (id, x/east, y/north, z/elevation).
pub fn parse_points(raw: &RawTable) -> Result<HashMap<String, TopoPoint>, ChError> {
    let (header, rows) = raw.rows.split_first().ok_or(ChError::Empty("points"))?;
    let norm: Vec<String> = header.iter().map(|h| normalize_header(h)).collect();
    let mut taken = vec![false; norm.len()];
    let mut col = |aliases: &[&str]| {
        let i = find(&norm, aliases, &taken);
        if let Some(i) = i {
            taken[i] = true;
        }
        i
    };
    let id = col(&[
        "punto",
        "pto",
        "id",
        "estacion",
        "estación",
        "station",
        "point",
        "ponto",
        "nombre",
        "name",
        "codigo",
        "código",
    ]);
    let x = col(&["x", "este", "e", "east", "easting", "coordx", "leste"]);
    let y = col(&["y", "norte", "n", "north", "northing", "coordy"]);
    let z = col(&[
        "z",
        "cota",
        "elevacion",
        "elevación",
        "elev",
        "rl",
        "elevation",
        "altitud",
    ]);
    let (Some(id), Some(x), Some(y), Some(z)) = (id, x, y, z) else {
        let missing = [("id", id), ("x", x), ("y", y), ("z", z)]
            .iter()
            .filter(|(_, c)| c.is_none())
            .map(|(k, _)| *k)
            .collect();
        return Err(ChError::MissingColumns {
            table: "points",
            missing,
            headers: header.clone(),
        });
    };
    let dc = raw.decimal_comma;
    let mut out = HashMap::new();
    for r in rows {
        let get = |i: usize| r.get(i).and_then(|s| parse_number(s, dc));
        if let (Some(name), Some(x), Some(y), Some(z)) = (
            r.get(id).map(|s| s.trim().to_string()).filter(|s| !s.is_empty()),
            get(x),
            get(y),
            get(z),
        ) {
            out.insert(key(&name), TopoPoint { x, y, z });
        }
    }
    if out.is_empty() {
        return Err(ChError::Empty("points"));
    }
    Ok(out)
}

fn key(id: &str) -> String {
    id.trim().to_lowercase()
}

/// One row of the channel sheet.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SheetRow {
    pub row: usize,
    pub fields: BTreeMap<ChField, String>,
    /// Columns that are not channel fields: numeric ones are grades, the rest
    /// geology attributes.
    pub extra: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Sheet {
    pub rows: Vec<SheetRow>,
    pub decimal_comma: bool,
    pub mapping: Vec<(ChField, String)>,
    /// Extra columns kept as grades (numeric) and as geology (text).
    pub grade_columns: Vec<String>,
    pub text_columns: Vec<String>,
}

/// Parses the channel sheet; `overrides` maps a field key to a header.
pub fn parse_sheet(raw: &RawTable, overrides: &BTreeMap<String, String>) -> Result<Sheet, ChError> {
    let (header, rows) = raw.rows.split_first().ok_or(ChError::Empty("channels"))?;
    let norm: Vec<String> = header.iter().map(|h| normalize_header(h)).collect();
    let mut cols: BTreeMap<ChField, usize> = BTreeMap::new();
    let mut taken = vec![false; norm.len()];
    for (k, h) in overrides {
        let (Some(f), false) = (ChField::from_key(k), h.trim().is_empty()) else {
            continue;
        };
        let hn = normalize_header(h);
        let i = norm
            .iter()
            .position(|n| *n == hn)
            .ok_or_else(|| ChError::UnknownHeader {
                field: k.clone(),
                header: h.clone(),
            })?;
        cols.insert(f, i);
        taken[i] = true;
    }
    for f in ChField::ALL {
        if cols.contains_key(&f) {
            continue;
        }
        if let Some(i) = find(&norm, f.aliases(), &taken) {
            cols.insert(f, i);
            taken[i] = true;
        }
    }
    let mut missing: Vec<&'static str> = [Channel, Point, Distance]
        .iter()
        .filter(|f| !cols.contains_key(f))
        .map(|f| f.key())
        .collect();
    if !cols.contains_key(&Width) && !(cols.contains_key(&From) && cols.contains_key(&To)) {
        missing.push("width (or from + to)");
    }
    if !missing.is_empty() {
        return Err(ChError::MissingColumns {
            table: "channels",
            missing,
            headers: header.clone(),
        });
    }
    if rows.is_empty() {
        return Err(ChError::Empty("channels"));
    }
    let extra_cols: Vec<usize> = (0..header.len())
        .filter(|i| !taken[*i] && !header[*i].trim().is_empty())
        .collect();
    let dc = raw.decimal_comma;
    let parsed: Vec<SheetRow> = rows
        .iter()
        .enumerate()
        .map(|(r, row)| SheetRow {
            row: r + 1,
            fields: cols
                .iter()
                .map(|(&f, &i)| (f, row.get(i).cloned().unwrap_or_default()))
                .collect(),
            extra: extra_cols
                .iter()
                .map(|&i| (header[i].trim().to_string(), row.get(i).cloned().unwrap_or_default()))
                .collect(),
        })
        .collect();
    // An extra column is a grade when every non-empty cell is a number
    // (detection limits like "<0.01" count as numbers).
    let numeric = |name: &String| {
        parsed
            .iter()
            .filter_map(|r| r.extra.get(name))
            .filter(|v| !v.trim().is_empty())
            .all(|v| crate::import::parse_assay_value(v, dc).is_some())
    };
    let names: Vec<String> = extra_cols.iter().map(|&i| header[i].trim().to_string()).collect();
    let (grade_columns, text_columns): (Vec<String>, Vec<String>) = names.into_iter().partition(numeric);
    Ok(Sheet {
        rows: parsed,
        decimal_comma: dc,
        mapping: cols.iter().map(|(&f, &i)| (f, header[i].clone())).collect(),
        grade_columns,
        text_columns,
    })
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Anchor {
    /// The channel is centred on the located position.
    Centre,
    /// The channel starts at the located position.
    Start,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Options {
    /// Prefix of the hole ids (`CH-`).
    pub prefix: String,
    pub anchor: Anchor,
    /// Added to the point elevation (m) when the sheet has no height.
    pub height: f64,
    /// Channel azimuth relative to the working direction (90 = perpendicular).
    pub azimuth_offset: f64,
    /// Default inclination (degrees, positive up).
    pub inclination: f64,
    /// Per-channel orientation set on screen: channel -> (azimuth, inclination).
    pub overrides: BTreeMap<String, (f64, f64)>,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            prefix: "CH-".into(),
            anchor: Anchor::Centre,
            height: 0.0,
            azimuth_offset: 90.0,
            inclination: 0.0,
            overrides: BTreeMap::new(),
        }
    }
}

/// A problem with one channel (it is skipped).
#[derive(Debug, Clone, PartialEq)]
pub struct Issue {
    pub channel: String,
    pub row: usize,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Converted {
    pub collars: Vec<Collar>,
    pub surveys: Vec<Survey>,
    pub assays: Vec<Assay>,
    pub geo: Vec<GeoLog>,
    pub samples: usize,
    pub issues: Vec<Issue>,
}

fn azimuth_between(a: TopoPoint, b: TopoPoint) -> Option<f64> {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    (dx.hypot(dy) > 1e-9).then(|| dx.atan2(dy).to_degrees().rem_euclid(360.0))
}

/// Converts the sheet into drillhole tables.
pub fn convert(sheet: &Sheet, points: &HashMap<String, TopoPoint>, opts: &Options) -> Converted {
    let dc = sheet.decimal_comma;
    let num = |r: &SheetRow, f: ChField| r.fields.get(&f).and_then(|s| parse_number(s, dc));
    let text = |r: &SheetRow, f: ChField| r.fields.get(&f).map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    // Group rows by channel, keeping the order of the file.
    let mut order: Vec<String> = Vec::new();
    let mut groups: HashMap<String, Vec<&SheetRow>> = HashMap::new();
    for r in &sheet.rows {
        let Some(id) = text(r, Channel) else { continue };
        let k = key(&id);
        if !groups.contains_key(&k) {
            order.push(id.clone());
        }
        groups.entry(k).or_default().push(r);
    }
    let mut out = Converted::default();
    'channels: for id in order {
        let rows = &groups[&key(&id)];
        let first = rows[0];
        let fail = |out: &mut Converted, msg: String| {
            out.issues.push(Issue {
                channel: id.clone(),
                row: first.row,
                message: msg,
            })
        };
        let Some(pname) = text(first, Point) else {
            fail(&mut out, "no topographic point".into());
            continue;
        };
        let Some(p) = points.get(&key(&pname)).copied() else {
            fail(
                &mut out,
                format!("topographic point '{pname}' is not in the points table"),
            );
            continue;
        };
        let Some(dist) = num(first, Distance) else {
            fail(&mut out, "no distance".into());
            continue;
        };
        // Direction of the working: towards another point, or an azimuth.
        let advance = match text(first, Toward) {
            Some(t) => match points.get(&key(&t)) {
                Some(q) => match azimuth_between(p, *q) {
                    Some(a) => a,
                    None => {
                        fail(&mut out, format!("points '{pname}' and '{t}' are at the same place"));
                        continue;
                    }
                },
                None => {
                    fail(&mut out, format!("topographic point '{t}' is not in the points table"));
                    continue;
                }
            },
            None => match num(first, Azimuth) {
                Some(a) => a,
                None => {
                    fail(
                        &mut out,
                        "no direction: give the next point or the azimuth of the working".into(),
                    );
                    continue;
                }
            },
        };
        // Samples: from/to, or consecutive widths.
        let mut intervals: Vec<(f64, f64, &SheetRow)> = Vec::new();
        let mut at = 0.0;
        for r in rows {
            let (f, t) = match (num(r, From), num(r, To)) {
                (Some(f), Some(t)) => (f, t),
                _ => match num(r, Width) {
                    Some(w) => (at, at + w),
                    None => {
                        fail(&mut out, format!("row {}: no sample width (or from/to)", r.row));
                        continue 'channels;
                    }
                },
            };
            if t <= f || t.is_nan() || f.is_nan() {
                fail(
                    &mut out,
                    format!("row {}: sample length must be positive ({f} - {t})", r.row),
                );
                continue 'channels;
            }
            intervals.push((f, t, r));
            at = t;
        }
        let length = intervals.iter().map(|i| i.1).fold(0.0, f64::max);
        // Orientation: set on screen, per channel in the sheet, or the default.
        let (az, inc) = match opts.overrides.get(&key(&id)) {
            Some(o) => *o,
            None => (
                num(first, ChannelAzimuth)
                    .unwrap_or(advance + opts.azimuth_offset)
                    .rem_euclid(360.0),
                num(first, ChannelInclination).unwrap_or(opts.inclination),
            ),
        };
        let (a, i) = (az.to_radians(), inc.to_radians());
        let dir = [i.cos() * a.sin(), i.cos() * a.cos(), i.sin()];
        let along = advance.to_radians();
        let h = num(first, Height).unwrap_or(opts.height);
        let centre = [p.x + dist * along.sin(), p.y + dist * along.cos(), p.z + h];
        let back = if opts.anchor == Anchor::Centre {
            length / 2.0
        } else {
            0.0
        };
        let start = [
            centre[0] - dir[0] * back,
            centre[1] - dir[1] * back,
            centre[2] - dir[2] * back,
        ];
        let hole = format!("{}{}", opts.prefix, id.trim());
        out.collars.push(Collar {
            hole_id: hole.clone(),
            x: start[0],
            y: start[1],
            z: start[2],
            max_depth: length,
            azimuth: Some(az),
            dip: Some(inc),
        });
        out.surveys.push(Survey {
            hole_id: hole.clone(),
            depth: 0.0,
            azimuth: az,
            dip: inc,
        });
        out.surveys.push(Survey {
            hole_id: hole.clone(),
            depth: length,
            azimuth: az,
            dip: inc,
        });
        for (f, t, r) in intervals {
            let mut attrs: BTreeMap<String, Option<f64>> = sheet
                .grade_columns
                .iter()
                .map(|g| {
                    (
                        g.to_lowercase(),
                        r.extra.get(g).and_then(|v| crate::import::parse_assay_value(v, dc)),
                    )
                })
                .collect();
            attrs.insert("potencia".into(), num(r, Thickness));
            attrs.insert("ancho".into(), Some(t - f));
            out.assays.push(Assay {
                hole_id: hole.clone(),
                from: f,
                to: t,
                attributes: attrs,
            });
            let mut geo: BTreeMap<String, String> = BTreeMap::new();
            for (k, f2) in [
                ("tipo", SampleType),
                ("labor", Working),
                ("muestra", Sample),
                ("posicion", Position),
            ] {
                if let Some(v) = text(r, f2) {
                    geo.insert(k.into(), v);
                }
            }
            for c in &sheet.text_columns {
                if let Some(v) = r.extra.get(c).map(|s| s.trim()).filter(|s| !s.is_empty()) {
                    geo.insert(c.to_lowercase(), v.to_string());
                }
            }
            if !geo.is_empty() {
                out.geo.push(GeoLog {
                    hole_id: hole.clone(),
                    from: f,
                    to: t,
                    attributes: geo,
                });
            }
            out.samples += 1;
        }
    }
    out
}

/// CSV text of the converted tables: `(collar, survey, assays, geo)`.
pub fn to_csv(c: &Converted) -> (String, String, String, String) {
    let q = |s: &str| {
        if s.contains([',', '"', '\n']) {
            format!("\"{}\"", s.replace('"', "\"\""))
        } else {
            s.to_string()
        }
    };
    let n = |v: f64| format!("{v}");
    let mut collar = String::from("HOLEID,X,Y,Z,LENGTH,AZIMUTH,DIP\n");
    for k in &c.collars {
        collar.push_str(&format!(
            "{},{},{},{},{},{},{}\n",
            q(&k.hole_id),
            n(k.x),
            n(k.y),
            n(k.z),
            n(k.max_depth),
            n(k.azimuth.unwrap_or(0.0)),
            n(k.dip.unwrap_or(0.0))
        ));
    }
    let mut survey = String::from("HOLEID,DEPTH,AZIMUTH,DIP\n");
    for s in &c.surveys {
        survey.push_str(&format!(
            "{},{},{},{}\n",
            q(&s.hole_id),
            n(s.depth),
            n(s.azimuth),
            n(s.dip)
        ));
    }
    let grade_names: Vec<String> = {
        let mut v: Vec<String> = c.assays.iter().flat_map(|a| a.attributes.keys().cloned()).collect();
        v.sort();
        v.dedup();
        v
    };
    let mut assays = format!(
        "HOLEID,FROM,TO,{}\n",
        grade_names.iter().map(|g| q(g)).collect::<Vec<_>>().join(",")
    );
    for a in &c.assays {
        let vals: Vec<String> = grade_names
            .iter()
            .map(|g| a.attributes.get(g).copied().flatten().map(n).unwrap_or_default())
            .collect();
        assays.push_str(&format!(
            "{},{},{},{}\n",
            q(&a.hole_id),
            n(a.from),
            n(a.to),
            vals.join(",")
        ));
    }
    let geo_names: Vec<String> = {
        let mut v: Vec<String> = c.geo.iter().flat_map(|g| g.attributes.keys().cloned()).collect();
        v.sort();
        v.dedup();
        v
    };
    let mut geo = format!(
        "HOLEID,FROM,TO,{}\n",
        geo_names.iter().map(|g| q(g)).collect::<Vec<_>>().join(",")
    );
    for g in &c.geo {
        let vals: Vec<String> = geo_names
            .iter()
            .map(|k| q(g.attributes.get(k).map_or("", |s| s.as_str())))
            .collect();
        geo.push_str(&format!(
            "{},{},{},{}\n",
            q(&g.hole_id),
            n(g.from),
            n(g.to),
            vals.join(",")
        ));
    }
    (collar, survey, assays, geo)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::desurvey::desurvey;

    fn raw(t: &str) -> RawTable {
        RawTable::from_csv_bytes(t.as_bytes())
    }

    const POINTS: &str = "PUNTO;ESTE;NORTE;COTA\nPT-1;1000;2000;3000\nPT-2;1000;2100;3000\nPT-3;1100;2100;3010\n";

    #[test]
    fn located_from_a_point_towards_the_next() {
        let pts = parse_points(&raw(POINTS)).unwrap();
        // Working runs north (PT-1 -> PT-2); channel 12 m from PT-1, two samples of 0.4 and 0.6 m.
        let sheet = parse_sheet(&raw("CANAL;PUNTO;HACIA;DISTANCIA;ANCHO;POTENCIA;TIPO;AU;AG;MUESTRA\nC1;PT-1;PT-2;12;0,4;0,8;CAJA;0,1;5;M1\nC1;PT-1;PT-2;12;0,6;0,8;VETA;8,5;120;M2\n"), &BTreeMap::new()).unwrap();
        assert_eq!(sheet.grade_columns, vec!["AU".to_string(), "AG".to_string()]);
        let c = convert(&sheet, &pts, &Options::default());
        assert!(c.issues.is_empty(), "{:?}", c.issues);
        let k = &c.collars[0];
        assert_eq!(k.hole_id, "CH-C1");
        // Perpendicular to a north working: azimuth 90, horizontal, centred at N 2012.
        assert_eq!((k.azimuth, k.dip, k.max_depth), (Some(90.0), Some(0.0), 1.0));
        assert!(
            (k.x - 999.5).abs() < 1e-9 && (k.y - 2012.0).abs() < 1e-9 && (k.z - 3000.0).abs() < 1e-9,
            "{k:?}"
        );
        assert_eq!(c.assays.len(), 2);
        assert_eq!((c.assays[1].from, c.assays[1].to), (0.4, 1.0));
        assert_eq!(c.assays[1].attributes["au"], Some(8.5));
        assert_eq!(c.assays[1].attributes["potencia"], Some(0.8));
        assert_eq!(c.geo[1].attributes["tipo"], "VETA");
        assert_eq!(c.geo[1].attributes["muestra"], "M2");
        // The desurveyed end is 1 m east of the start.
        let trace = desurvey(k, &c.surveys);
        let end = trace.last().unwrap().xyz;
        assert!(
            (end[0] - 1000.5).abs() < 1e-6 && (end[1] - 2012.0).abs() < 1e-6 && (end[2] - 3000.0).abs() < 1e-6,
            "{end:?}"
        );
    }

    #[test]
    fn azimuth_from_to_overrides_and_issues() {
        let pts = parse_points(&raw(POINTS)).unwrap();
        let sheet = parse_sheet(
            &raw("canal,punto,azimut,distancia,desde,hasta,altura,az_canal,inc_canal,cu%\nA,PT-2,90,5,0,1.5,1.2,,,1.2\nB,PT-3,0,0,0,2,,45,30,0.5\nC,PT-9,0,5,0,1,,,,0.3\nD,PT-1,,5,0,1,,,,0.3\nE,PT-1,0,3,1,1,,,,0.2\n"),
            &BTreeMap::new(),
        )
        .unwrap();
        let mut opts = Options {
            anchor: Anchor::Start,
            ..Default::default()
        };
        opts.overrides.insert("a".into(), (10.0, -5.0));
        let c = convert(&sheet, &pts, &opts);
        assert_eq!(c.collars.len(), 2);
        // A: working east from PT-2, 5 m along -> (1005, 2100), height 1.2; orientation set on screen.
        let a = &c.collars[0];
        assert!((a.x - 1005.0).abs() < 1e-9 && (a.y - 2100.0).abs() < 1e-9 && (a.z - 3001.2).abs() < 1e-9);
        assert_eq!((a.azimuth, a.dip), (Some(10.0), Some(-5.0)));
        // B: its own azimuth and inclination.
        assert_eq!((c.collars[1].azimuth, c.collars[1].dip), (Some(45.0), Some(30.0)));
        let msgs: Vec<&str> = c.issues.iter().map(|i| i.channel.as_str()).collect();
        assert_eq!(msgs, vec!["C", "D", "E"]);
        assert!(
            c.issues[0].message.contains("PT-9")
                && c.issues[1].message.contains("direction")
                && c.issues[2].message.contains("positive")
        );
    }

    #[test]
    fn required_columns_and_csv() {
        let e = parse_sheet(&raw("canal,distancia\nA,1\n"), &BTreeMap::new()).unwrap_err();
        assert!(
            matches!(e, ChError::MissingColumns { ref missing, .. } if missing.contains(&"point") && missing.iter().any(|m| m.starts_with("width")))
        );
        let ov: BTreeMap<String, String> = [
            ("point".to_string(), "est".to_string()),
            ("width".to_string(), "w".to_string()),
        ]
        .into();
        assert!(parse_sheet(&raw("canal,est,distancia,w\nA,PT-1,1,0.5\n"), &ov).is_ok());
        assert!(parse_points(&raw("a,b\n1,2\n")).is_err());
        let pts = parse_points(&raw(POINTS)).unwrap();
        let sheet = parse_sheet(
            &raw("canal,punto,hacia,distancia,ancho,au\nX,PT-1,PT-2,1,0.5,<0.01\n"),
            &BTreeMap::new(),
        )
        .unwrap();
        let c = convert(&sheet, &pts, &Options::default());
        let (collar, survey, assays, _) = to_csv(&c);
        assert!(collar.starts_with("HOLEID,X,Y,Z,LENGTH,AZIMUTH,DIP\nCH-X,"));
        assert_eq!(survey.lines().count(), 3);
        assert!(assays.contains("ancho") && assays.contains("potencia"));
        // The CSV reads back with the drillhole importer.
        let back =
            crate::import::parse_collars(&RawTable::from_csv_bytes(collar.as_bytes()), &Default::default()).unwrap();
        assert_eq!(back[0].hole_id, "CH-X");
    }
}
