use std::fmt::{self, Display, Formatter};

/// Declared physical endpoint/format for a typed artifact.
///
/// This enum is intentionally dependency-free. It can say `file::yaml` or
/// `sql::sqlite` without importing serde_yaml, toml, rusqlite, chain RPC
/// clients, or any other backend. Feature-backed adapters can live above this
/// declaration layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArtifactEndpoint {
    FileJson,
    FileYaml,
    FileToml,
    FileTxt,
    FileJsonl,
    FileXml,
    FileMarkdown,
    FileCbor,
    FileDocx,
    FilePdf,
    FileNpy,
    FileNpz,

    /// SQLite table endpoint. The database path should be resolved by a
    /// separate descriptor/policy; the table name is part of the declared route.
    SqliteTable { table: &'static str },

    /// Declared external chain target. This is metadata only in pathgen core;
    /// signing/broadcasting belongs in a custody-aware adapter layer.
    ChainTarget { route_id: &'static str },
}

impl ArtifactEndpoint {
    pub const fn is_file(self) -> bool {
        matches!(
            self,
            ArtifactEndpoint::FileJson
                | ArtifactEndpoint::FileYaml
                | ArtifactEndpoint::FileToml
                | ArtifactEndpoint::FileTxt
                | ArtifactEndpoint::FileJsonl
                | ArtifactEndpoint::FileXml
                | ArtifactEndpoint::FileMarkdown
                | ArtifactEndpoint::FileCbor
                | ArtifactEndpoint::FileDocx
                | ArtifactEndpoint::FilePdf
                | ArtifactEndpoint::FileNpy
                | ArtifactEndpoint::FileNpz
        )
    }

    pub const fn label(self) -> &'static str {
        match self {
            ArtifactEndpoint::FileJson => "file::json",
            ArtifactEndpoint::FileYaml => "file::yaml",
            ArtifactEndpoint::FileToml => "file::toml",
            ArtifactEndpoint::FileTxt => "file::txt",
            ArtifactEndpoint::FileJsonl => "file::jsonl",
            ArtifactEndpoint::FileXml => "file::xml",
            ArtifactEndpoint::FileMarkdown => "file::markdown",
            ArtifactEndpoint::FileCbor => "file::cbor",
            ArtifactEndpoint::FileDocx => "file::docx",
            ArtifactEndpoint::FilePdf => "file::pdf",
            ArtifactEndpoint::FileNpy => "file::npy",
            ArtifactEndpoint::FileNpz => "file::npz",
            ArtifactEndpoint::SqliteTable { .. } => "sql::sqlite",
            ArtifactEndpoint::ChainTarget { .. } => "chain::target",
        }
    }

    pub const fn extension(self) -> Option<&'static str> {
        match self {
            ArtifactEndpoint::FileJson => Some("json"),
            ArtifactEndpoint::FileYaml => Some("yaml"),
            ArtifactEndpoint::FileToml => Some("toml"),
            ArtifactEndpoint::FileTxt => Some("txt"),
            ArtifactEndpoint::FileJsonl => Some("jsonl"),
            ArtifactEndpoint::FileXml => Some("xml"),
            ArtifactEndpoint::FileMarkdown => Some("md"),
            ArtifactEndpoint::FileCbor => Some("cbor"),
            ArtifactEndpoint::FileDocx => Some("docx"),
            ArtifactEndpoint::FilePdf => Some("pdf"),
            ArtifactEndpoint::FileNpy => Some("npy"),
            ArtifactEndpoint::FileNpz => Some("npz"),
            ArtifactEndpoint::SqliteTable { .. } | ArtifactEndpoint::ChainTarget { .. } => None,
        }
    }
}

impl Display for ArtifactEndpoint {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            ArtifactEndpoint::SqliteTable { table } => write!(f, "sql::sqlite(table={})", table),
            ArtifactEndpoint::ChainTarget { route_id } => write!(f, "chain::target(route={})", route_id),
            other => write!(f, "{}", (*other).label()),
        }
    }
}

/// Low-bit operation family used by the compiled artifact-op handle.
///
/// Three bits are enough for the first eight endpoint families. Detailed
/// routing information still lives in the descriptor; the tag is only a fast
/// operation-kind discriminator.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArtifactOpTag {
    FileJson = 0,
    FileYaml = 1,
    FileToml = 2,
    FileTxt = 3,
    FileJsonl = 4,
    ExtendedFile = 5,
    Sqlite = 6,
    Chain = 7,
}

impl ArtifactOpTag {
    pub const LOW_BITS: usize = 3;
    pub const MASK: usize = (1usize << Self::LOW_BITS) - 1;

    pub const fn as_usize(self) -> usize {
        self as usize
    }

    pub const fn from_endpoint(endpoint: ArtifactEndpoint) -> Self {
        match endpoint {
            ArtifactEndpoint::FileJson => ArtifactOpTag::FileJson,
            ArtifactEndpoint::FileYaml => ArtifactOpTag::FileYaml,
            ArtifactEndpoint::FileToml => ArtifactOpTag::FileToml,
            ArtifactEndpoint::FileTxt => ArtifactOpTag::FileTxt,
            ArtifactEndpoint::FileJsonl => ArtifactOpTag::FileJsonl,
            ArtifactEndpoint::FileXml
            | ArtifactEndpoint::FileMarkdown
            | ArtifactEndpoint::FileCbor
            | ArtifactEndpoint::FileDocx
            | ArtifactEndpoint::FilePdf
            | ArtifactEndpoint::FileNpy
            | ArtifactEndpoint::FileNpz => ArtifactOpTag::ExtendedFile,
            ArtifactEndpoint::SqliteTable { .. } => ArtifactOpTag::Sqlite,
            ArtifactEndpoint::ChainTarget { .. } => ArtifactOpTag::Chain,
        }
    }
}
