use serde::Deserialize;

/// Shell script to run
///
/// This is used for both check and run actions within a service step.
#[derive(Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum ScriptConfig {
    /// Single string with multiple lines
    Multiline(String),
    /// Multiple strings in an array
    Array(Vec<String>),
    /// A single boolean value
    Boolean(bool),
    /// An empty value
    None,
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
    fn script_multiline() {
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
    fn script_array() {
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
    fn script_override() {
        let value = true;
        let dh: DataHolder = toml::from_str("commands = true").expect("unable to read config");

        match dh.commands {
            ScriptConfig::Boolean(test_value) => {
                assert_eq!(test_value, value);
            }
            _ => assert!(false),
        }
    }
}
