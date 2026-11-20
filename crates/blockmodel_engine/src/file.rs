//! Binary block model file (`.ombm`, version 1) and CSV export.
//!
//! ```text
//! OMBM1\n
//! geometry=ox,oy,oz,dx,dy,dz,nx,ny,nz,rotation\n
//! domains=name1|name2\n
//! columns=col1,col2,...\n
//! end\n
//! domain: n * u8 | fraction: n * f32 LE | each column in header order: n * f32 LE
//! ```

use std::io::{self, Read, Write};

use crate::geometry::BlockGeometry;
use crate::model::BlockModel;

const MAGIC: &str = "OMBM1\n";

#[derive(Debug)]
pub enum FileError {
    Io(io::Error),
    Format(String),
}

impl std::fmt::Display for FileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "{e}"),
            Self::Format(m) => write!(f, "invalid block model file: {m}"),
        }
    }
}
impl std::error::Error for FileError {}
impl From<io::Error> for FileError {
    fn from(e: io::Error) -> Self {
        FileError::Io(e)
    }
}

fn bad<T>(m: &str) -> Result<T, FileError> {
    Err(FileError::Format(m.to_string()))
}

pub fn write_model(m: &BlockModel, w: &mut impl Write) -> Result<(), FileError> {
    let g = &m.geometry;
    w.write_all(MAGIC.as_bytes())?;
    writeln!(
        w,
        "geometry={},{},{},{},{},{},{},{},{},{}",
        g.origin[0], g.origin[1], g.origin[2], g.size[0], g.size[1], g.size[2], g.n[0], g.n[1], g.n[2], g.rotation
    )?;
    writeln!(
        w,
        "domains={}",
        m.domain_names
            .iter()
            .map(|s| s.replace(['|', '\n'], " "))
            .collect::<Vec<_>>()
            .join("|")
    )?;
    writeln!(w, "columns={}", m.columns.keys().cloned().collect::<Vec<_>>().join(","))?;
    writeln!(w, "end")?;
    w.write_all(&m.domain)?;
    let mut buf = Vec::with_capacity(m.len() * 4);
    for f in &m.fraction {
        buf.extend_from_slice(&f.to_le_bytes());
    }
    w.write_all(&buf)?;
    for col in m.columns.values() {
        buf.clear();
        for v in col {
            buf.extend_from_slice(&v.to_le_bytes());
        }
        w.write_all(&buf)?;
    }
    Ok(())
}

pub fn read_model(r: &mut impl Read) -> Result<BlockModel, FileError> {
    let mut all = Vec::new();
    r.read_to_end(&mut all)?;
    if !all.starts_with(MAGIC.as_bytes()) {
        return bad("not an OMBM1 file");
    }
    // Header lines up to "end\n".
    let mut pos = MAGIC.len();
    let mut lines: Vec<String> = Vec::new();
    loop {
        let Some(nl) = all[pos..].iter().position(|&b| b == b'\n') else {
            return bad("truncated header");
        };
        let line = String::from_utf8_lossy(&all[pos..pos + nl]).to_string();
        pos += nl + 1;
        if line == "end" {
            break;
        }
        lines.push(line);
        if lines.len() > 16 {
            return bad("header too long");
        }
    }
    let get = |k: &str| {
        lines
            .iter()
            .find_map(|l| l.strip_prefix(&format!("{k}=")))
            .ok_or_else(|| FileError::Format(format!("missing {k}")))
    };
    let nums: Vec<f64> = get("geometry")?
        .split(',')
        .map(|s| s.parse::<f64>().map_err(|_| FileError::Format("bad geometry".into())))
        .collect::<Result<_, _>>()?;
    if nums.len() != 10 {
        return bad("geometry needs 10 values");
    }
    let geometry = BlockGeometry {
        origin: [nums[0], nums[1], nums[2]],
        size: [nums[3], nums[4], nums[5]],
        n: [nums[6] as usize, nums[7] as usize, nums[8] as usize],
        rotation: nums[9],
    };
    if geometry.size.iter().any(|v| !(v.is_finite() && *v > 0.0))
        || !geometry.rotation.is_finite()
        || geometry.origin.iter().any(|v| !v.is_finite())
    {
        return bad("invalid geometry values");
    }
    let n = match geometry.n[0]
        .checked_mul(geometry.n[1])
        .and_then(|v| v.checked_mul(geometry.n[2]))
    {
        Some(n) if n > 0 && n <= crate::geometry::MAX_BLOCKS => n,
        _ => return bad("block count out of range"),
    };
    let domain_names: Vec<String> = get("domains")?
        .split('|')
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect();
    let names: Vec<String> = get("columns")?
        .split(',')
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect();
    // n <= MAX_BLOCKS keeps these products far from overflow.
    let need = n * 5 + names.len() * n * 4;
    if all.len() - pos != need {
        return bad(&format!("expected {need} data bytes, found {}", all.len() - pos));
    }
    let f32s = |bytes: &[u8]| -> Vec<f32> {
        let (chunks, _) = bytes.as_chunks::<4>();
        chunks.iter().map(|c| f32::from_le_bytes(*c)).collect()
    };
    let domain = all[pos..pos + n].to_vec();
    if domain.iter().any(|&d| d as usize > domain_names.len()) {
        return bad("block domain id without a domain name");
    }
    pos += n;
    let fraction = f32s(&all[pos..pos + n * 4]);
    pos += n * 4;
    let mut columns = std::collections::BTreeMap::new();
    for name in names {
        columns.insert(name, f32s(&all[pos..pos + n * 4]));
        pos += n * 4;
    }
    Ok(BlockModel {
        geometry,
        domain_names,
        domain,
        fraction,
        columns,
    })
}

pub fn save(m: &BlockModel, path: &std::path::Path) -> Result<(), FileError> {
    let tmp = path.with_extension("ombm.tmp");
    {
        let mut f = io::BufWriter::new(std::fs::File::create(&tmp)?);
        write_model(m, &mut f)?;
        f.flush()?;
    }
    std::fs::rename(&tmp, path)?;
    Ok(())
}

pub fn load(path: &std::path::Path) -> Result<BlockModel, FileError> {
    read_model(&mut io::BufReader::new(std::fs::File::open(path)?))
}

/// CSV with block centres, size, domain name, fraction and every column.
pub fn write_csv(m: &BlockModel, w: &mut impl Write) -> io::Result<()> {
    let g = &m.geometry;
    write!(w, "x,y,z,dx,dy,dz,domain,fraction")?;
    for c in m.columns.keys() {
        write!(w, ",{c}")?;
    }
    writeln!(w)?;
    for i in 0..m.len() {
        if m.domain[i] == 0 {
            continue;
        }
        let c = g.center(i);
        write!(
            w,
            "{:.3},{:.3},{:.3},{},{},{},{},{:.4}",
            c[0],
            c[1],
            c[2],
            g.size[0],
            g.size[1],
            g.size[2],
            m.domain_name(m.domain[i]),
            m.fraction[i]
        )?;
        for col in m.columns.values() {
            let v = col[i];
            if v.is_nan() {
                write!(w, ",")?;
            } else {
                write!(w, ",{v}")?;
            }
        }
        writeln!(w)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> BlockModel {
        let mut m = BlockModel::new(BlockGeometry {
            origin: [96_440.0, 87_550.0, 1_704.0],
            size: [10.0, 10.0, 12.0],
            n: [3, 2, 2],
            rotation: 325.0,
        });
        m.domain_names = vec!["Óxidos".into(), "Sulfuros".into()];
        for i in 0..m.len() {
            m.domain[i] = (i % 3) as u8;
            m.fraction[i] = i as f32 / 20.0;
        }
        m.column_mut("au_ok")
            .iter_mut()
            .enumerate()
            .for_each(|(i, v)| *v = if i % 4 == 0 { f32::NAN } else { i as f32 * 0.37 });
        m.column_mut("density").iter_mut().for_each(|v| *v = 2.7);
        m
    }

    #[test]
    fn binary_roundtrip_is_lossless_including_nan() {
        let m = sample();
        let mut buf = Vec::new();
        write_model(&m, &mut buf).unwrap();
        let back = read_model(&mut buf.as_slice()).unwrap();
        assert_eq!(back.geometry, m.geometry);
        assert_eq!(back.domain_names, m.domain_names);
        assert_eq!(back.domain, m.domain);
        assert_eq!(back.fraction, m.fraction);
        for (k, v) in &m.columns {
            let b = &back.columns[k];
            assert!(v.iter().zip(b).all(|(x, y)| x.to_bits() == y.to_bits()), "{k}");
        }
    }

    #[test]
    fn corrupt_files_are_rejected_not_misread() {
        let mut buf = Vec::new();
        write_model(&sample(), &mut buf).unwrap();
        assert!(matches!(
            read_model(&mut &buf[..buf.len() - 3]),
            Err(FileError::Format(_))
        ));
        assert!(matches!(read_model(&mut b"NOPE".as_slice()), Err(FileError::Format(_))));
        let mut extra = buf.clone();
        extra.push(0);
        assert!(read_model(&mut extra.as_slice()).is_err());
    }

    #[test]
    fn unknown_domain_ids_and_absurd_headers_are_rejected() {
        let mut m = sample();
        m.domain[0] = 3; // only 2 names
        let mut buf = Vec::new();
        write_model(&m, &mut buf).unwrap();
        assert!(matches!(read_model(&mut buf.as_slice()), Err(FileError::Format(_))));
        let huge = b"OMBM1\ngeometry=0,0,0,1,1,1,4294967296,4294967296,4294967296,0\ndomains=A\ncolumns=\nend\n";
        assert!(matches!(read_model(&mut huge.as_slice()), Err(FileError::Format(_))));
        let zero = b"OMBM1\ngeometry=0,0,0,0,1,1,1,1,1,0\ndomains=A\ncolumns=\nend\n\x01\x00\x00\x80\x3f";
        assert!(matches!(read_model(&mut zero.as_slice()), Err(FileError::Format(_))));
    }

    #[test]
    fn save_load_and_csv() {
        let dir = std::env::temp_dir().join(format!("ombm_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("m.ombm");
        save(&sample(), &path).unwrap();
        assert_eq!(load(&path).unwrap().geometry, sample().geometry);
        let mut csv = Vec::new();
        write_csv(&sample(), &mut csv).unwrap();
        let text = String::from_utf8(csv).unwrap();
        let mut lines = text.lines();
        assert_eq!(lines.next().unwrap(), "x,y,z,dx,dy,dz,domain,fraction,au_ok,density");
        // Only assigned blocks (8 of 12 have domain != 0).
        assert_eq!(lines.count(), sample().assigned());
        std::fs::remove_dir_all(dir).ok();
    }
}
