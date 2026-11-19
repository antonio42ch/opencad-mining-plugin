use std::fmt;

/// Which input table an error refers to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TableKind {
    Collar,
    Survey,
    Assay,
    Geo,
}

impl TableKind {
    pub fn key(self) -> &'static str {
        match self {
            Self::Collar => "collar",
            Self::Survey => "survey",
            Self::Assay => "assay",
            Self::Geo => "geo",
        }
    }
}

/// Typed import failures; the UI layer translates them.
#[derive(Debug, Clone, PartialEq)]
pub enum ImportError {
    Empty {
        table: TableKind,
    },
    MissingIdColumn {
        table: TableKind,
        headers: Vec<String>,
    },
    MissingColumn {
        table: TableKind,
        field: &'static str,
        headers: Vec<String>,
    },
    /// A manual mapping names a header that is not in the file.
    UnknownMappedColumn {
        table: TableKind,
        field: &'static str,
        column: String,
    },
    Io {
        table: TableKind,
        message: String,
    },
    Spreadsheet {
        table: TableKind,
        message: String,
    },
}

impl ImportError {
    pub fn table(&self) -> TableKind {
        match self {
            Self::Empty { table }
            | Self::MissingIdColumn { table, .. }
            | Self::MissingColumn { table, .. }
            | Self::UnknownMappedColumn { table, .. }
            | Self::Io { table, .. }
            | Self::Spreadsheet { table, .. } => *table,
        }
    }

    /// Headers found in the file, when the error is a mapping problem (lets the
    /// UI offer a column-mapping dialog).
    pub fn headers(&self) -> Option<&[String]> {
        match self {
            Self::MissingIdColumn { headers, .. } | Self::MissingColumn { headers, .. } => Some(headers),
            _ => None,
        }
    }
}

impl fmt::Display for ImportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty { table } => write!(f, "{}: empty file", table.key()),
            Self::MissingIdColumn { table, .. } => {
                write!(f, "{}: hole id column not found", table.key())
            }
            Self::MissingColumn { table, field, .. } => {
                write!(f, "{}: column '{field}' not found", table.key())
            }
            Self::UnknownMappedColumn { table, field, column } => {
                write!(
                    f,
                    "{}: mapped column '{column}' for '{field}' does not exist",
                    table.key()
                )
            }
            Self::Io { table, message } => write!(f, "{}: {message}", table.key()),
            Self::Spreadsheet { table, message } => write!(f, "{}: {message}", table.key()),
        }
    }
}

impl std::error::Error for ImportError {}
