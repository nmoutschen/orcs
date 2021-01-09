use crate::{
    config::{RecipeConfig, RecipeStepConfig, ScriptConfig, ServiceConfig, ServiceStepConfig},
    Error, Result,
};
use std::collections::{HashMap, HashSet};

/// Service
///
/// This contains a resolved version of the Service, after parsing recipes and
/// steps.
pub struct Service {
    /// Name of the service
    pub name: String,

    /// Steps and their internal representation.
    ///
    /// During a run, we need to retrieve all `ServiceStep` pairs that match
    /// the run.
    steps: HashMap<String, ServiceStep>,
}

impl Service {
    /// Create a service from a configuration file
    ///
    /// This returns a builder to process the recipes mentioned in the
    /// configuration file, as the final `Service` struct is not aware of the
    /// recipes.
    pub fn from_config<'a, 'b>(name: &'a str, config: &'b ServiceConfig) -> ServiceBuilder<'a, 'b> {
        ServiceBuilder {
            name: name,
            steps: config.steps.iter().map(|(step_name, step_config)| (step_name, step_config.into())).collect(),
        }
    }

    // Retrieve a step
    pub fn get_step(&self, step_name: &str) -> Option<&ServiceStep> {
        self.steps.get(step_name)
    }
}

/// Builder for a Service
pub struct ServiceBuilder<'a, 'b> {
    name: &'a str,

    steps: HashMap<&'b String, ServiceStepBuilder<'b>>,
}

impl<'a, 'b> ServiceBuilder<'a, 'b> {
    /// Inject a recipe into the builder
    ///
    /// If a step doesn't exist in the `ServiceBuilder`, this will inject it
    /// with the values from the `RecipeConfig`.
    pub fn with_recipe(&mut self, recipe: &'b RecipeConfig) -> &mut ServiceBuilder<'a, 'b> {
        for (step_name, step_config) in &recipe.steps {
            // Case 1: the step doesn't exist, so we just override it
            if !self.steps.contains_key(step_name) {
                self.steps.insert(step_name, step_config.into());
            }
            // Case 2: the service exists, but check or run are not set
            let step_builder = self.steps.get_mut(step_name).expect("failed to get step builder");
            step_builder.with_recipe(step_config);
        }

        self
    }

    /// Build into an owned `Service`
    pub fn build(self) -> Service {
        Service {
            name: self.name.to_string(),
            steps: self
                .steps
                .iter()
                .map(|(step_name, step_builder)| (
                    (*step_name).to_owned(),
                    step_builder.build(self.name, step_name),
                ))
                .collect(),
        }
    }
}

#[derive(Clone)]
/// Unique step:service pair
pub struct ServiceStep {
    /// Name of the pair in 'step:service' format.
    pub name: String,

    depends_on: Vec<String>,

    check: Script,
    run: Script,
}

pub struct ServiceStepBuilder<'a> {
    depends_on: Option<&'a Vec<String>>,
    check: &'a ScriptConfig,
    run: &'a ScriptConfig,
}

impl<'a> From<&'a ServiceStepConfig> for ServiceStepBuilder<'a> {
    fn from(config: &'a ServiceStepConfig) -> Self {
        Self {
            depends_on: Some(&config.depends_on),
            check: &config.check,
            run: &config.run,
        }
    }
}

impl<'a> From<&'a RecipeStepConfig> for ServiceStepBuilder<'a> {
    fn from(config: &'a RecipeStepConfig) -> Self {
        Self {
            depends_on: None,
            check: &config.check,
            run: &config.run,
        }
    }
}

impl<'a> ServiceStepBuilder<'a> {
    /// Update the `ServiceStepBuilder` with values from a `RecipeStepConfig`
    /// if the builder doesn't contain values for check or run and the recipe
    /// does.
    pub fn with_recipe(&mut self, config: &'a RecipeStepConfig) -> &mut Self {
        if self.check == &ScriptConfig::None && config.check != ScriptConfig::None {
            self.check = &config.check;
        }
        if self.run == &ScriptConfig::None && config.run != ScriptConfig::None {
            self.run = &config.run;
        }

        self
    }

    /// Build into an owned `ServiceStep`
    pub fn build(&self, service_name: &str, step_name: &str) -> ServiceStep {
        ServiceStep {
            name: format!("{}:{}", step_name, service_name),
            depends_on: match self.depends_on {
                Some(deps) => deps.clone(),
                None => Vec::new(),
            },
            check: self.check.into(),
            run: self.run.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Script {
    Script(String),
    Override(bool),
    None,
}

impl From<&ScriptConfig> for Script {
    fn from(config: &ScriptConfig) -> Self {
        match config {
            ScriptConfig::Array(val) => Self::Script(val.join("\n")),
            ScriptConfig::Multiline(val) => Self::Script(val.to_owned()),
            ScriptConfig::Boolean(val) => Self::Override(val.to_owned()),
            ScriptConfig::None => Self::None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_builder() {
        // Starting with a simple config
        let service_config = ServiceConfig {
            steps: vec![(String::from("my-step"), ServiceStepConfig {
                run: ScriptConfig::Boolean(true),
                ..Default::default()
            })].into_iter().collect(),
            ..Default::default()
        };

        let service_builder = Service::from_config("my-service", &service_config);

        let service = service_builder.build();
        let service_step = service.steps.get("my-step").expect("failed to get step");

        assert_eq!(service.name, "my-service");
        assert_eq!(service_step.depends_on, Vec::new() as Vec<String>);
        assert_eq!(service_step.run, Script::Override(true));
        assert_eq!(service_step.check, Script::None);
    }

    #[test]
    fn service_builder_with_recipe() {
        // Starting with a simple config
        let service_config = ServiceConfig {
            recipes: vec![String::from("my-recipe")],

            ..Default::default()
        };

        // Recipe
        let recipe_config = RecipeConfig {
            steps: vec![(String::from("my-step"), RecipeStepConfig {
                run: ScriptConfig::Boolean(true),
                ..Default::default()
            })].into_iter().collect(),
            ..Default::default()
        };

        // Build the service
        let mut service_builder = Service::from_config("my-service", &service_config);
        service_builder.with_recipe(&recipe_config);

        let service = service_builder.build();
        let service_step = service.steps.get("my-step").expect("failed to get step");

        // Assertions
        assert_eq!(service.name, "my-service");
        assert_eq!(service_step.depends_on, Vec::new() as Vec<String>);
        assert_eq!(service_step.run, Script::Override(true));
        assert_eq!(service_step.check, Script::None);
    }

    #[test]
    fn service_builder_with_recipe_2() {
        // Starting with a simple config
        let service_config = ServiceConfig {
            recipes: vec![String::from("my-recipe1"), String::from("my-recipe2")],

            ..Default::default()
        };

        // Recipes
        let recipe_config1 = RecipeConfig {
            steps: vec![(String::from("my-step1"), RecipeStepConfig {
                run: ScriptConfig::Boolean(true),
                ..Default::default()
            })].into_iter().collect(),
            ..Default::default()
        };
        let recipe_config2 = RecipeConfig {
            steps: vec![
                (String::from("my-step1"), RecipeStepConfig {
                    run: ScriptConfig::Boolean(false),
                    check: ScriptConfig::Boolean(false),
                    ..Default::default()
                }),
                (String::from("my-step2"), RecipeStepConfig {
                    run: ScriptConfig::Boolean(false),
                    ..Default::default()
                })
            ].into_iter().collect(),
            ..Default::default()
        };

        // Build the service
        let mut service_builder = Service::from_config("my-service", &service_config);
        service_builder.with_recipe(&recipe_config1);
        service_builder.with_recipe(&recipe_config2);

        let service = service_builder.build();
        let service_step1 = service.steps.get("my-step1").expect("failed to get step");
        let service_step2 = service.steps.get("my-step2").expect("failed to get step");

        // Assertions
        assert_eq!(service.name, "my-service");
        assert_eq!(service_step1.depends_on, Vec::new() as Vec<String>);
        assert_eq!(service_step1.run, Script::Override(true));
        assert_eq!(service_step1.check, Script::Override(false));
        assert_eq!(service_step2.depends_on, Vec::new() as Vec<String>);
        assert_eq!(service_step2.run, Script::Override(false));
        assert_eq!(service_step2.check, Script::None);
    }

    #[test]
    fn service_step_builder() {
        // Starting with a simple config
        let step_config = ServiceStepConfig {
            run: ScriptConfig::Boolean(true),
            ..Default::default()
        };

        // Create the step builder
        let step_builder: ServiceStepBuilder = (&step_config).into();

        // Build the ServiceStep
        let step = step_builder.build("my-service", "my-step");

        // Assertions
        assert_eq!(step.name, "my-step:my-service");
        assert_eq!(step.run, Script::Override(true));
        assert_eq!(step.check, Script::None);
    }

    #[test]
    fn service_step_builder_with_recipe() {
        // Starting with an empty config
        let step_config = ServiceStepConfig {
            ..Default::default()
        };
        let recipe_config = RecipeStepConfig {
            run: ScriptConfig::Boolean(true),
            ..Default::default()
        };

        // Create the step builder
        let mut step_builder: ServiceStepBuilder = (&step_config).into();

        // Apply the recipe
        step_builder.with_recipe(&recipe_config);

        // Build the ServiceStep
        let step = step_builder.build("my-service", "my-step");

        // Assertions
        assert_eq!(step.name, "my-step:my-service");
        assert_eq!(step.run, Script::Override(true));
        assert_eq!(step.check, Script::None);
    }

    #[test]
    fn service_step_builder_with_recipe_2() {
        // Starting with an empty config
        let step_config = ServiceStepConfig {
            ..Default::default()
        };
        let recipe_config1 = RecipeStepConfig {
            run: ScriptConfig::Boolean(true),
            ..Default::default()
        };
        let recipe_config2 = RecipeStepConfig {
            run: ScriptConfig::Boolean(false),
            check: ScriptConfig::Boolean(false),
        };

        // Create the step builder
        let mut step_builder: ServiceStepBuilder = (&step_config).into();

        // Apply the recipe
        step_builder.with_recipe(&recipe_config1);
        step_builder.with_recipe(&recipe_config2);

        // Build the ServiceStep
        let step = step_builder.build("my-service", "my-step");

        // Assertions
        assert_eq!(step.name, "my-step:my-service");
        assert_eq!(step.run, Script::Override(true));
        assert_eq!(step.check, Script::Override(false));
    }

    #[test]
    fn script_from_multiline() {
        let value = "a\nb\nc";
        let config = ScriptConfig::Multiline(String::from("a\nb\nc"));

        let script = Script::from(&config);

        match script {
            Script::Script(test_value) => assert_eq!(value, test_value),
            _ => assert!(false),
        }
    }

    #[test]
    fn script_from_array() {
        let value = "a\nb\nc";
        let config = ScriptConfig::Array(vec![
            String::from("a"),
            String::from("b"),
            String::from("c"),
        ]);

        let script = Script::from(&config);

        match script {
            Script::Script(test_value) => assert_eq!(value, test_value),
            _ => assert!(false),
        }
    }

    #[test]
    fn script_from_boolean() {
        let value = true;
        let config = ScriptConfig::Boolean(true);

        let script = Script::from(&config);

        match script {
            Script::Override(test_value) => assert_eq!(value, test_value),
            _ => assert!(false),
        }
    }
}
