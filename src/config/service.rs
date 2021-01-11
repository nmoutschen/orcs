use super::ScriptConfig;
use serde::Deserialize;
use std::collections::HashMap;

/// Config file of a service
#[derive(Default, Deserialize)]
pub struct ServiceConfig {
    /// Storing commands and dependencies for each step for the service
    #[serde(default)]
    pub steps: HashMap<String, ServiceStepConfig>,

    /// Array of recipes for this service
    #[serde(default)]
    pub recipes: Vec<String>,
}

/// Step in a service config file
#[derive(Default, Deserialize)]
pub struct ServiceStepConfig {
    /// List of step:service pairs that this specific step:service pair
    /// depends on.
    #[serde(default)]
    pub depends_on: Vec<String>,

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
        let config = ServiceConfig::default();

        assert_eq!(config.steps.len(), 0);
        assert_eq!(config.recipes.len(), 0);
    }

    #[test]
    fn deserialize() {
        let data = "
            recipes = [\"my-recipe\"]

            [steps.my-step]
            depends_on = [\"a\", \"b\", \"c\"]
            run = true
            check = true
        ";

        let config: ServiceConfig = toml::from_str(data).expect("failed to deserialize data");

        assert_eq!(config.recipes, ["my-recipe"]);
        assert!(config.steps.contains_key("my-step"));

        let step = config.steps.get("my-step").expect("failed to get step");
        assert_eq!(step.depends_on, ["a", "b", "c"]);
        assert_eq!(step.run, ScriptConfig::Boolean(true));
        assert_eq!(step.check, ScriptConfig::Boolean(true));
    }

    #[test]
    fn default_step() {
        let step = ServiceStepConfig::default();

        assert_eq!(step.depends_on.len(), 0);
        assert_eq!(step.check, ScriptConfig::None);
        assert_eq!(step.run, ScriptConfig::None);
    }

    #[test]
    fn deserialize_step() {
        let data = "
            depends_on = [\"a\", \"b\", \"c\"]
            run = true
            check = true
        ";
        let step: ServiceStepConfig = toml::from_str(data).expect("unable to deserialize data");

        assert_eq!(step.depends_on, ["a", "b", "c"]);
        assert_eq!(step.run, ScriptConfig::Boolean(true));
        assert_eq!(step.check, ScriptConfig::Boolean(true));
    }
}
