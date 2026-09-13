use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub enum LoadError {
    WrongExtension(PathBuf),
    Io { path: PathBuf, source: io::Error },
}

impl fmt::Display for LoadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongExtension(path) => write!(
                formatter,
                "Aerofyl source files must use `.fyl`: {}",
                path.display()
            ),
            Self::Io { path, source } => {
                write!(formatter, "could not read {}: {source}", path.display())
            }
        }
    }
}

impl std::error::Error for LoadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::WrongExtension(_) => None,
        }
    }
}

pub fn load_source(path: &Path) -> Result<String, LoadError> {
    if path.extension().and_then(|extension| extension.to_str()) != Some("fyl") {
        return Err(LoadError::WrongExtension(path.to_owned()));
    }
    fs::read_to_string(path).map_err(|source| LoadError::Io {
        path: path.to_owned(),
        source,
    })
}
