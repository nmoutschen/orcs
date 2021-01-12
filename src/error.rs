use std::fmt;
use std::path::PathBuf;

pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Errors specific to Orcs
#[derive(Debug)]
pub enum Error {
    // File errors
    ConfigFileNotFound {
        path: PathBuf,
    },
    CannotReadConfigFile {
        path: PathBuf,
        source: std::io::Error,
    },
    CannotParseConfigFile {
        path: PathBuf,
        source: toml::de::Error,
    },

    // Project errors
    ProjectIsNotGitRepo {
        path: PathBuf,
        source: git2::Error,
    },

    // Service errors
    MissingRecipes {
        names: Vec<String>,
    },
    MissingStep {
        name: String,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            // File errors
            Self::ConfigFileNotFound { path } => {
                write!(f, "config file not found: '{}'", path.display())
            }
            Self::CannotReadConfigFile { path, source } => write!(
                f,
                "cannot open config file '{}': {}",
                path.display(),
                source
            ),
            Self::CannotParseConfigFile { path, source } => write!(
                f,
                "cannot parse config file '{}': {}",
                path.display(),
                source
            ),
            // Project errors
            Self::ProjectIsNotGitRepo { path, source } => write!(
                f,
                "project is not a git repo at '{}': {}",
                path.display(),
                source
            ),
            // Service errors
            Self::MissingRecipes { names } => {
                write!(f, "missing one or more recipes: '{}'", names.join(","))
            }
            Self::MissingStep { name } => {
                write!(f, "missing a step in the project configuration: '{}'", name)
            }
        }
    }
}
