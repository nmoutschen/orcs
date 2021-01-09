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

// TODO: add tests
// #[cfg(test)]
// mod tests {
//     use super::*;
// }
