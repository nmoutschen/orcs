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

// TODO: add tests
// #[cfg(test)]
// mod tests {
//     use super::*;
// }
