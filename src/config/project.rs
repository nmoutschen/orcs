use serde::Deserialize;
use std::collections::HashMap;

const DEFAULT_CONTAINER_IMAGE: &str = "ubuntu:20.04";

/// Representation of the project configuration file
#[derive(Debug, Default, Deserialize)]
pub struct ProjectConfig {
    pub name: String,

    #[serde(default)]
    pub steps: HashMap<String, ProjectStepConfig>,

    #[serde(default)]
    pub options: ProjectOptions,
}

/// Represent the configuration for a project step
///
/// This doesn't contain any default actions, but just the dependencies from
/// that step to other steps.
#[derive(Debug, Default, Deserialize)]
pub struct ProjectStepConfig {
    /// List of dependencies for that step.
    ///
    /// This should contain an array of step names that should be run before
    /// this step.
    #[serde(default)]
    pub depends_on: Vec<String>,

    /// Whether this step should be skipped unless explicitely mentioned.
    ///
    /// This is useful for cleanup steps or steps that shouldn't be run
    /// automatically.
    #[serde(default)]
    pub skip_run: bool,

    /// Whether we should run this stage when we detected a change within a
    /// service.
    #[serde(default)]
    pub on_changed: ProjectStepOnChanged,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub enum ProjectStepOnChanged {
    /// Don't do anything for this step on changed
    #[serde(rename = "skip")]
    Skip,
    /// Do a check first, and perform a run only if needed
    #[serde(rename = "check_first")]
    CheckFirst,
    /// Always run on changed
    #[serde(rename = "run")]
    Run,
}

impl Default for ProjectStepOnChanged {
    fn default() -> Self {
        Self::Run
    }
}

/// All options and flags for a project
#[derive(Debug, Deserialize, Default)]
pub struct ProjectOptions {
    /// Name of the container image to use
    ///
    /// By default, we use the `ubuntu:20.04` container image.
    #[serde(default = "default_container_image")]
    pub container_image: String,
}

#[inline]
fn default_container_image() -> String {
    String::from(DEFAULT_CONTAINER_IMAGE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize() {
        let data = "
        name = \"my-project\"

        [options]

        container_image = \"my-container\"

        [steps.my-step]
        depends_on = [\"a\", \"b\", \"c\"]
        skip_run = true
        on_changed = \"run\"
        ";

        let config: ProjectConfig = toml::from_str(data).expect("failed to deserialize data");

        // Name
        assert_eq!(config.name, "my-project");

        // Options
        assert_eq!(config.options.container_image, "my-container");

        // Steps
        assert!(config.steps.contains_key("my-step"));
        let step = config.steps.get("my-step").expect("failed to get step");
        assert_eq!(step.depends_on, ["a", "b", "c"]);
        assert_eq!(step.skip_run, true);
        assert_eq!(step.on_changed, ProjectStepOnChanged::Run);
    }

    #[test]
    fn default_step() {
        let step: ProjectStepConfig = Default::default();

        assert_eq!(step.depends_on, Vec::new() as Vec<String>);
        assert_eq!(step.skip_run, false);
        assert_eq!(step.on_changed, ProjectStepOnChanged::Run);
    }

    #[test]
    fn deserialize_step() {
        let data = "
            depends_on = [\"a\", \"b\", \"c\"]
            skip_run = true
            on_changed = \"check_first\"
        ";
        let step: ProjectStepConfig = toml::from_str(data).expect("unable to deserialize data");

        assert_eq!(step.depends_on, ["a", "b", "c"]);
        assert_eq!(step.skip_run, true);
        assert_eq!(step.on_changed, ProjectStepOnChanged::CheckFirst);
    }

    #[test]
    fn deserialize_step_on_changed() {
        let test_cases = [
            ("skip", ProjectStepOnChanged::Skip),
            ("run", ProjectStepOnChanged::Run),
            ("check_first", ProjectStepOnChanged::CheckFirst),
        ];

        for test_case in test_cases.iter() {
            let val: ProjectStepOnChanged = toml::from_str(&format!("\"{}\"", test_case.0))
                .expect("unable to deserialize data");
            assert_eq!(val, test_case.1);
        }
    }
}
