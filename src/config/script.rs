use serde::Deserialize;

/// Shell script to run
///
/// This is used for both check and run actions within a service step.
#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum ScriptConfig {
    /// Single string with multiple lines
    Multiline(String),
    /// Multiple strings in an array
    Array(Vec<String>),
    /// A single boolean value
    Boolean(bool),
    /// An empty value
    ///
    /// This is the default value if the related property (usually 'run' or
    /// 'check') is not specified in a configuration file.
    None,
}

impl ScriptConfig {
    /// Check if this contains an empty value
    ///
    /// There are multiple scenarios where this could contain an empty value:
    /// * it's a default value (`Self::None`)
    /// * it contains an empty array (`Self::Multiline`)
    /// * it contains an empty string (`Self::Array`)
    pub fn is_empty(&self) -> bool {
        match self {
            Self::Multiline(val) => val.is_empty(),
            Self::Array(val) => val.is_empty(),
            Self::Boolean(_) => false,
            Self::None => true,
        }
    }
}

impl Default for ScriptConfig {
    fn default() -> Self {
        Self::None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Deserialize)]
    struct DataHolder {
        pub commands: ScriptConfig,
    }

    #[test]
    fn deserialize_multiline() {
        let value = "this\nis\na\nvalue";
        let dh: DataHolder = toml::from_str(&format!("commands = \"\"\"{}\"\"\"", value))
            .expect("unable to read config");

        match dh.commands {
            ScriptConfig::Multiline(test_value) => {
                assert_eq!(test_value, value);
            }
            _ => assert!(false),
        }
    }

    #[test]
    fn deserialize_array() {
        let value = "this\nis\na\nvalue".split("\n").collect::<Vec<_>>();
        let dh: DataHolder = toml::from_str(&format!("commands = [\"{}\"]", value.join("\",\"")))
            .expect("unable to read config");

        match dh.commands {
            ScriptConfig::Array(test_value) => {
                assert_eq!(test_value, value);
            }
            _ => assert!(false),
        }
    }

    #[test]
    fn deserialize_boolean() {
        let value = true;
        let dh: DataHolder = toml::from_str("commands = true").expect("unable to read config");

        match dh.commands {
            ScriptConfig::Boolean(test_value) => {
                assert_eq!(test_value, value);
            }
            _ => assert!(false),
        }
    }

    #[test]
    fn default_value() {
        let config = ScriptConfig::default();

        assert_eq!(config, ScriptConfig::None);
    }
}
