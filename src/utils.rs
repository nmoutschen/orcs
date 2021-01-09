use crate::{Error, Result};
use serde::de::DeserializeOwned;
use std::fs::File;
use std::io::prelude::*;
use std::path::PathBuf;

/// Load a TOML file into the given structure and return an appropriate error
/// if the file cannot be loaded.
pub fn load_config<P, T>(path: P) -> Result<T>
where
    P: Into<PathBuf>,
    T: DeserializeOwned,
{
    let path = path.into();

    // Check if the file exists
    if !path.is_file() {
        return Err(Error::ConfigFileNotFound { path });
    }

    // Read the file content
    let mut file = File::open(&path).map_err(|source| Error::CannotReadConfigFile {
        path: path.clone(),
        source,
    })?;
    let mut data = String::new();
    file.read_to_string(&mut data)
        .map_err(|source| Error::CannotReadConfigFile {
            path: path.clone(),
            source,
        })?;

    // Parse the config file and return the result
    toml::from_str(&data).map_err(|source| Error::CannotParseConfigFile {
        path: path.clone(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;
    use tempfile::tempdir;

    #[derive(Deserialize, PartialEq, Debug)]
    struct TestConfigData {
        message: String,
    }

    #[test]
    fn test_load_config() {
        let value = TestConfigData {
            message: String::from("this is a test"),
        };
        let data = "message = \"this is a test\"";
        // Need to use a tempdir instead of tempfile as File doesn't expose a
        // path.
        let dir = tempdir().expect("failed to create temporary folder");
        let path = dir.path().join("test.txt");

        let mut file = File::create(&path).expect("failed to create file");
        file.write_all(data.as_bytes())
            .expect("failed to write test data");

        let result: TestConfigData = load_config(&path).expect("failed to open file");
        assert_eq!(result, value);
    }
}
