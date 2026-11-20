//! Definitions: operations, parameters, panels.

/// A text in the three UI languages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct L {
    pub es: &'static str,
    pub en: &'static str,
    pub pt: &'static str,
}

impl L {
    pub const fn new(es: &'static str, en: &'static str, pt: &'static str) -> L {
        L { es, en, pt }
    }
    /// Same text in every language (symbols, acronyms).
    pub const fn same(t: &'static str) -> L {
        L { es: t, en: t, pt: t }
    }
    pub fn get(&self, lang: Lang) -> &'static str {
        match lang {
            Lang::Es => self.es,
            Lang::En => self.en,
            Lang::Pt => self.pt,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    Es,
    En,
    Pt,
}

/// One option of a choice: stable key (arguments, JSON) + label.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Opt {
    pub key: &'static str,
    pub label: L,
}

pub const fn opt(key: &'static str, label: L) -> Opt {
    Opt { key, label }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Kind {
    Number {
        min: Option<f64>,
        max: Option<f64>,
        step: f64,
    },
    Integer {
        min: Option<i64>,
        max: Option<i64>,
    },
    Bool,
    Text,
    /// A file path; `extensions` filter the picker (lowercase, no dot).
    Path {
        extensions: &'static [&'static str],
    },
    /// Stored in the settings as the option index.
    Choice {
        options: &'static [Opt],
    },
    /// `x,y` or `x,y,z` (world coordinates).
    Point,
    /// One entity handle (hex).
    Handle,
    /// Entity handles, comma-separated (hex).
    Handles,
    /// Rows of cells; `columns` name them.
    Table {
        columns: &'static [L],
    },
}

impl Kind {
    pub const fn num(min: Option<f64>, max: Option<f64>, step: f64) -> Kind {
        Kind::Number { min, max, step }
    }
    pub fn type_name(&self) -> &'static str {
        match self {
            Kind::Number { .. } => "number",
            Kind::Integer { .. } => "integer",
            Kind::Bool => "boolean",
            Kind::Text => "text",
            Kind::Path { .. } => "path",
            Kind::Choice { .. } => "choice",
            Kind::Point => "point",
            Kind::Handle => "handle",
            Kind::Handles => "handles",
            Kind::Table { .. } => "table",
        }
    }
}

/// File types read by the importers.
pub const TABLE_FILES: &[&str] = &["csv", "txt", "tsv", "xlsx", "xls", "ods"];

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Param {
    /// Argument name (`[a-z0-9_]`).
    pub key: &'static str,
    pub kind: Kind,
    /// JSON pointer into the project settings. Such a parameter is a
    /// **setting**: its value comes from the settings when not given, and a
    /// given value is stored for this and later runs (as the panel does).
    pub setting: Option<&'static str>,
    /// Must be given (only for non-settings).
    pub required: bool,
    /// For AI clients and the guide (English).
    pub help: &'static str,
    pub label: L,
}

pub const fn setting(key: &'static str, kind: Kind, ptr: &'static str, label: L, help: &'static str) -> Param {
    Param {
        key,
        kind,
        setting: Some(ptr),
        required: false,
        help,
        label,
    }
}

pub const fn input(key: &'static str, kind: Kind, required: bool, label: L, help: &'static str) -> Param {
    Param {
        key,
        kind,
        setting: None,
        required,
        help,
        label,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Group {
    Project,
    Drillholes,
    Workings,
    Axes,
    Sections,
    Geology,
    Resources,
    Geotechnics,
}

impl Group {
    pub fn title(self) -> &'static str {
        match self {
            Group::Project => "Project",
            Group::Drillholes => "Drillholes",
            Group::Workings => "Underground workings",
            Group::Axes => "Axis designer",
            Group::Sections => "Sections",
            Group::Geology => "Geological mapping",
            Group::Resources => "Resource estimation",
            Group::Geotechnics => "Geotechnics",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpKind {
    /// Changes the drawing or the project data.
    Action,
    /// Reads only.
    Query,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OpDef {
    /// Tool name (`[a-z0-9_]`).
    pub name: &'static str,
    /// Command typed in the command line (`MIN_...`).
    pub command: &'static str,
    /// Older command names that still work.
    pub aliases: &'static [&'static str],
    /// Button / ribbon text.
    pub label: L,
    pub group: Group,
    pub kind: OpKind,
    /// What it does and when to use it (English, for AI clients).
    pub summary: &'static str,
    pub params: &'static [Param],
    /// Parameters accepted without `key=` in this order (`-` skips one).
    pub positional: &'static [&'static str],
    /// The `data` object of a successful result (English).
    pub result: &'static str,
}

impl OpDef {
    pub fn param(&self, key: &str) -> Option<&'static Param> {
        self.params.iter().find(|p| p.key == key)
    }
}

/// Layout of a panel; values and labels come from the operations.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Item {
    /// A setting of an operation: `(op name, param key)`.
    Param(&'static str, &'static str),
    /// Runs the operation with the panel's values.
    Button(&'static str),
    Section {
        id: &'static str,
        title: L,
        items: &'static [Item],
        collapsed: bool,
    },
    Row(&'static [Item]),
    Info(L),
    /// Status label filled by the commands.
    Status(&'static str),
    /// SVG chart filled by the commands.
    Chart(&'static str),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PanelSpec {
    pub id: &'static str,
    pub title: L,
    /// Command that toggles the panel.
    pub command: &'static str,
    pub items: &'static [Item],
}

impl PanelSpec {
    /// Every item, depth first.
    pub fn visit(&self, f: &mut dyn FnMut(&Item)) {
        fn walk(items: &[Item], f: &mut dyn FnMut(&Item)) {
            for it in items {
                f(it);
                match it {
                    Item::Section { items, .. } | Item::Row(items) => walk(items, f),
                    _ => {}
                }
            }
        }
        walk(self.items, f);
    }
}

/// Control id of a setting in a panel: the pointer with `.` (`gt.md_radii.0`).
pub fn control_id(ptr: &str) -> String {
    ptr.trim_start_matches('/').replace('/', ".")
}
