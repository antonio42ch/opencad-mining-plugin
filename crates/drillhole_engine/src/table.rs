//! Raw tabular input: decoding, delimiter detection, CSV parsing, XLSX.

use crate::error::{ImportError, TableKind};

/// Rows of trimmed-or-not strings; the first row is the header.
#[derive(Debug, Clone, PartialEq)]
pub struct RawTable {
    pub rows: Vec<Vec<String>>,
    /// `true` when numbers use a decimal comma (`;`-separated CSV from
    /// Spanish/Portuguese locales).
    pub decimal_comma: bool,
}

/// UTF-8 (with or without BOM); otherwise Windows-1252 / Latin-1, which is
/// what Excel writes on Spanish-language Windows.
pub fn decode(bytes: &[u8]) -> String {
    let bytes = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
    match std::str::from_utf8(bytes) {
        Ok(s) => s.to_string(),
        Err(_) => bytes.iter().map(|&b| cp1252(b)).collect(),
    }
}

fn cp1252(b: u8) -> char {
    const HI: [char; 32] = [
        '€', '\u{81}', '‚', 'ƒ', '„', '…', '†', '‡', 'ˆ', '‰', 'Š', '‹', 'Œ', '\u{8d}', 'Ž', '\u{8f}', '\u{90}', '‘',
        '’', '“', '”', '•', '–', '—', '˜', '™', 'š', '›', 'œ', '\u{9d}', 'ž', 'Ÿ',
    ];
    if (0x80..0xA0).contains(&b) {
        HI[(b - 0x80) as usize]
    } else {
        b as char
    }
}

/// Picks the delimiter that occurs most often (outside quotes) in the header line.
pub fn detect_delimiter(text: &str) -> char {
    let header = text.lines().find(|l| !l.trim().is_empty()).unwrap_or("");
    let mut best = (',', 0usize);
    for d in [',', ';', '\t', '|'] {
        let mut in_q = false;
        let n = header
            .chars()
            .filter(|&c| {
                if c == '"' {
                    in_q = !in_q;
                }
                !in_q && c == d
            })
            .count();
        if n > best.1 {
            best = (d, n);
        }
    }
    best.0
}

/// RFC-4180-ish parser: quotes, escaped quotes, CRLF. Blank rows dropped.
pub fn parse_delimited(text: &str, delim: char) -> Vec<Vec<String>> {
    let mut rows = Vec::new();
    let mut row: Vec<String> = Vec::new();
    let mut field = String::new();
    let mut in_quotes = false;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if in_quotes {
            match c {
                '"' if chars.peek() == Some(&'"') => {
                    field.push('"');
                    chars.next();
                }
                '"' => in_quotes = false,
                _ => field.push(c),
            }
        } else if c == '"' {
            in_quotes = true;
        } else if c == delim {
            row.push(std::mem::take(&mut field));
        } else if c == '\n' {
            row.push(std::mem::take(&mut field));
            rows.push(std::mem::take(&mut row));
        } else if c != '\r' {
            field.push(c);
        }
    }
    if !field.is_empty() || !row.is_empty() {
        row.push(field);
        rows.push(row);
    }
    rows.retain(|r| r.iter().any(|f| !f.trim().is_empty()));
    rows
}

impl RawTable {
    /// Decodes and parses delimited text bytes.
    pub fn from_csv_bytes(bytes: &[u8]) -> Self {
        let text = decode(bytes);
        let delim = detect_delimiter(&text);
        RawTable {
            rows: parse_delimited(&text, delim),
            decimal_comma: delim != ',',
        }
    }

    /// Reads a file by extension: `.xlsx/.xlsm/.xls/.ods` need the `xlsx`
    /// feature, everything else is treated as delimited text.
    pub fn from_path(path: &std::path::Path, table: TableKind) -> Result<Self, ImportError> {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if matches!(ext.as_str(), "xlsx" | "xlsm" | "xls" | "ods") {
            return Self::from_spreadsheet(path, table);
        }
        let bytes = std::fs::read(path).map_err(|e| ImportError::Io {
            table,
            message: e.to_string(),
        })?;
        Ok(Self::from_csv_bytes(&bytes))
    }

    #[cfg(feature = "xlsx")]
    fn from_spreadsheet(path: &std::path::Path, table: TableKind) -> Result<Self, ImportError> {
        use calamine::{open_workbook_auto, Data, Reader};
        let err = |m: String| ImportError::Spreadsheet { table, message: m };
        let mut wb = open_workbook_auto(path).map_err(|e| err(e.to_string()))?;
        let name = wb
            .sheet_names()
            .first()
            .cloned()
            .ok_or_else(|| err("workbook has no sheets".into()))?;
        let range = wb.worksheet_range(&name).map_err(|e| err(e.to_string()))?;
        let rows = range
            .rows()
            .map(|r| {
                r.iter()
                    .map(|c| match c {
                        Data::Empty => String::new(),
                        Data::Float(f) => f.to_string(),
                        Data::Int(i) => i.to_string(),
                        other => other.to_string(),
                    })
                    .collect::<Vec<_>>()
            })
            .filter(|r| r.iter().any(|f| !f.trim().is_empty()))
            .collect();
        Ok(RawTable {
            rows,
            decimal_comma: false,
        })
    }

    #[cfg(not(feature = "xlsx"))]
    fn from_spreadsheet(_path: &std::path::Path, table: TableKind) -> Result<Self, ImportError> {
        Err(ImportError::Spreadsheet {
            table,
            message: "spreadsheet support not compiled in (feature 'xlsx')".into(),
        })
    }
}

/// Header normalisation: lowercase, drop `-`, `_`, `.` and spaces. `%` is kept.
pub fn normalize_header(h: &str) -> String {
    h.trim()
        .to_lowercase()
        .chars()
        .filter(|c| !matches!(c, '-' | '_' | ' ' | '.'))
        .collect()
}

/// Number parsing honouring the table's decimal separator.
pub fn parse_number(s: &str, decimal_comma: bool) -> Option<f64> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    let v = if decimal_comma {
        s.replace('.', "").replace(',', ".").parse::<f64>()
    } else {
        s.parse::<f64>()
    };
    v.ok().filter(|v| v.is_finite())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quotes_and_crlf() {
        let r = parse_delimited("a,\"b,c\",d\r\n1,\"x\"\"y\",3\r\n\r\n", ',');
        assert_eq!(r, vec![vec!["a", "b,c", "d"], vec!["1", "x\"y", "3"]]);
    }

    #[test]
    fn spanish_excel_csv() {
        // Latin-1 'ó' (0xF3), semicolons, decimal comma.
        let bytes = b"HOLEID;ELEVACI\xd3N;X\nDH1;1.234,5;10\n";
        let t = RawTable::from_csv_bytes(bytes);
        assert!(t.decimal_comma);
        assert_eq!(t.rows[0][1], "ELEVACIÓN");
        assert_eq!(parse_number(&t.rows[1][1], t.decimal_comma), Some(1234.5));
    }

    #[test]
    fn delimiter_detection() {
        assert_eq!(detect_delimiter("a\tb\tc\n"), '\t');
        assert_eq!(detect_delimiter("\"a;b\",c,d\n"), ',');
    }

    #[test]
    fn headers() {
        assert_eq!(normalize_header("HOLE-ID"), "holeid");
        assert_eq!(normalize_header("Cu %"), "cu%");
    }
}
