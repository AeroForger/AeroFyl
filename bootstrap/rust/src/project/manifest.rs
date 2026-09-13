use std::path::PathBuf;

/// Lossless project configuration source.
///
/// Aerofyl specifies TOML as the format, but does not yet specify a file name or
/// any keys. Consequently the bootstrap can transport manifest text but cannot
/// interpret it without inventing a schema.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectManifest {
    pub path: PathBuf,
    pub toml_source: String,
}

impl ProjectManifest {
    pub fn new(path: impl Into<PathBuf>, toml_source: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            toml_source: toml_source.into(),
        }
    }
}
