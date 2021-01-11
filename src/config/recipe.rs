use super::ScriptConfig;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Default, Deserialize)]
pub struct RecipeConfig {
    #[serde(default)]
    pub steps: HashMap<String, RecipeStepConfig>,
}

/// Step in a recipe config file
///
/// This is similar to a `ServiceStepConfig` with the exception that
/// `RecipeStepConfig` doesn't support `depends_on`.
#[derive(Default, Deserialize)]
pub struct RecipeStepConfig {
    /// Shell script to run on a 'check'
    #[serde(default)]
    pub check: ScriptConfig,

    /// Shell script to run on a 'run'
    #[serde(default)]
    pub run: ScriptConfig,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default() {
        let config = RecipeConfig::default();

        assert_eq!(config.steps.len(), 0);
    }

    #[test]
    fn deserialize() {
        let data = "
            [steps.my-step]
            depends_on = [\"a\", \"b\", \"c\"]
            run = true
            check = true
        ";

        let config: RecipeConfig = toml::from_str(data).expect("failed to deserialize data");

        assert!(config.steps.contains_key("my-step"));

        let step = config.steps.get("my-step").expect("failed to get step");
        assert_eq!(step.run, ScriptConfig::Boolean(true));
        assert_eq!(step.check, ScriptConfig::Boolean(true));
    }

    #[test]
    fn default_step() {
        let step_config = RecipeStepConfig::default();

        assert_eq!(step_config.check, ScriptConfig::None);
        assert_eq!(step_config.run, ScriptConfig::None);
    }

    #[test]
    fn deserialize_step() {
        let data = "
            run = true
            check = true
        ";
        let step: RecipeStepConfig = toml::from_str(data).expect("unable to deserialize data");

        assert_eq!(step.run, ScriptConfig::Boolean(true));
        assert_eq!(step.check, ScriptConfig::Boolean(true));
    }
}
