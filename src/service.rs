use crate::config::{
    ProjectStepConfig, ProjectStepOnChanged, RecipeConfig, RecipeStepConfig, ScriptConfig,
    ServiceConfig, ServiceStepConfig,
};
use crate::{Error, Result};
use std::collections::HashMap;

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
    pub fn from_config<'a, 'b, 'c>(
        name: &'a str,
        project_steps: &'c HashMap<String, ProjectStepConfig>,
        config: &'b ServiceConfig,
    ) -> Result<ServiceBuilder<'a, 'b, 'c>> {
        Ok(ServiceBuilder {
            name,
            steps: config
                .steps
                .iter()
                .map(|(step_name, step_config)| {
                    Ok((
                        step_name,
                        ServiceStep::from_service_config(
                            step_name,
                            name,
                            project_steps
                                .get(step_name)
                                .ok_or_else(|| Error::MissingStep {
                                    name: step_name.to_string(),
                                })?,
                            step_config,
                        ),
                    ))
                })
                .collect::<Result<HashMap<_, _>, Error>>()?,
        })
    }

    // Retrieve a `ServiceStep` pair if it exists
    pub fn get_step(&self, step_name: &str) -> Option<&ServiceStep> {
        self.steps.get(step_name)
    }
}

/// Builder for a Service
///
/// This is returned from the `Service::from_config` call and contains a method
/// to process recipes one at a time.
pub struct ServiceBuilder<'a, 'b, 'c> {
    name: &'a str,

    steps: HashMap<&'b String, ServiceStepBuilder<'b, 'c>>,
}

impl<'a, 'b, 'c> ServiceBuilder<'a, 'b, 'c> {
    /// Inject a recipe into the builder
    ///
    /// If a step doesn't exist in the `ServiceBuilder`, this will inject it
    /// with the values from the `RecipeConfig`.
    pub fn with_recipe(
        &mut self,
        project_steps: &'c HashMap<String, ProjectStepConfig>,
        recipe: &'b RecipeConfig,
    ) -> Result<&mut ServiceBuilder<'a, 'b, 'c>> {
        for (step_name, step_config) in &recipe.steps {
            // Case 1: the step doesn't exist, so we just override it
            // TODO: add error for missing project step
            if !self.steps.contains_key(step_name) {
                let builder = ServiceStep::from_recipe_config(
                    step_name,
                    self.name,
                    project_steps
                        .get(step_name)
                        .ok_or_else(|| Error::MissingStep {
                            name: step_name.to_string(),
                        })?,
                    step_config,
                );
                self.steps.insert(step_name, builder);
            }
            // Case 2: the service exists, but check or run are not set
            let step_builder = self
                .steps
                .get_mut(step_name)
                .expect("failed to get step builder");
            step_builder.with_recipe(step_config);
        }

        Ok(self)
    }

    /// Build into an owned `Service`
    pub fn build(self) -> Service {
        Service {
            name: self.name.to_string(),
            steps: self
                .steps
                .iter()
                .map(|(step_name, step_builder)| ((*step_name).to_owned(), step_builder.build()))
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

    on_changed: StepOnChanged,

    check: Script,
    run: Script,
}

impl ServiceStep {
    pub fn from_service_config<'a, 'b>(
        step_name: &str,
        service_name: &str,
        pconfig: &'b ProjectStepConfig,
        sconfig: &'a ServiceStepConfig,
    ) -> ServiceStepBuilder<'a, 'b> {
        ServiceStepBuilder {
            name: format!("{}:{}", step_name, service_name),
            depends_on: pconfig
                .depends_on
                .iter()
                .map(|dep_step_name| format!("{}:{}", dep_step_name, service_name))
                .chain(
                    sconfig
                        .depends_on
                        .iter()
                        .map(|dep_service_name| format!("{}:{}", step_name, dep_service_name)),
                )
                .collect(),

            on_changed: &pconfig.on_changed,

            check: &sconfig.check,
            run: &sconfig.run,
        }
    }

    pub fn from_recipe_config<'a, 'b>(
        step_name: &str,
        service_name: &str,
        pconfig: &'b ProjectStepConfig,
        rconfig: &'a RecipeStepConfig,
    ) -> ServiceStepBuilder<'a, 'b> {
        ServiceStepBuilder {
            name: format!("{}:{}", step_name, service_name),
            depends_on: pconfig
                .depends_on
                .iter()
                .map(|dep_step_name| format!("{}:{}", dep_step_name, service_name))
                .collect(),

            on_changed: &pconfig.on_changed,

            check: &rconfig.check,
            run: &rconfig.run,
        }
    }
}

pub struct ServiceStepBuilder<'a, 'b> {
    name: String,

    depends_on: Vec<String>,

    on_changed: &'b ProjectStepOnChanged,

    check: &'a ScriptConfig,
    run: &'a ScriptConfig,
}

impl<'a, 'b> ServiceStepBuilder<'a, 'b> {
    /// Update the `ServiceStepBuilder` with values from a `RecipeStepConfig`
    /// if the builder doesn't contain values for check or run and the recipe
    /// does.
    pub fn with_recipe(&mut self, config: &'a RecipeStepConfig) -> &mut Self {
        if self.check.is_empty() && !config.check.is_empty() {
            self.check = &config.check;
        }
        if self.run.is_empty() && !config.run.is_empty() {
            self.run = &config.run;
        }

        self
    }

    /// Build into an owned `ServiceStep`
    pub fn build(&self) -> ServiceStep {
        ServiceStep {
            name: self.name.to_owned(),
            depends_on: self.depends_on.to_owned(),
            on_changed: self.on_changed.into(),
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

#[derive(Clone)]
pub enum StepOnChanged {
    Skip,
    CheckFirst,
    Run,
}

impl From<&ProjectStepOnChanged> for StepOnChanged {
    fn from(source: &ProjectStepOnChanged) -> Self {
        match source {
            ProjectStepOnChanged::Skip => Self::Skip,
            ProjectStepOnChanged::CheckFirst => Self::CheckFirst,
            ProjectStepOnChanged::Run => Self::Run,
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
            steps: vec![(
                String::from("my-step"),
                ServiceStepConfig {
                    run: ScriptConfig::Boolean(true),
                    ..Default::default()
                },
            )]
            .into_iter()
            .collect(),
            ..Default::default()
        };

        // Project Step Configs
        let mut project_step_configs: HashMap<String, ProjectStepConfig> = HashMap::new();
        project_step_configs.insert(String::from("my-step"), ProjectStepConfig::default());

        // Build a service
        let service_builder =
            Service::from_config("my-service", &project_step_configs, &service_config)
                .expect("failed to create builder");
        let service = service_builder.build();

        // Get the step
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
            steps: vec![(
                String::from("my-step"),
                RecipeStepConfig {
                    run: ScriptConfig::Boolean(true),
                    ..Default::default()
                },
            )]
            .into_iter()
            .collect(),
            ..Default::default()
        };

        // Project Step Configs
        let mut project_step_configs: HashMap<String, ProjectStepConfig> = HashMap::new();
        project_step_configs.insert(String::from("my-step"), ProjectStepConfig::default());

        // Build the service
        let mut service_builder =
            Service::from_config("my-service", &project_step_configs, &service_config)
                .expect("failed to create builder");
        service_builder
            .with_recipe(&project_step_configs, &recipe_config)
            .expect("failed to use recipe");

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
            steps: vec![(
                String::from("my-step1"),
                RecipeStepConfig {
                    run: ScriptConfig::Boolean(true),
                    ..Default::default()
                },
            )]
            .into_iter()
            .collect(),
            ..Default::default()
        };
        let recipe_config2 = RecipeConfig {
            steps: vec![
                (
                    String::from("my-step1"),
                    RecipeStepConfig {
                        run: ScriptConfig::Boolean(false),
                        check: ScriptConfig::Boolean(false),
                        ..Default::default()
                    },
                ),
                (
                    String::from("my-step2"),
                    RecipeStepConfig {
                        run: ScriptConfig::Boolean(false),
                        ..Default::default()
                    },
                ),
            ]
            .into_iter()
            .collect(),
            ..Default::default()
        };

        // Project Step Config
        let mut project_step_configs: HashMap<String, ProjectStepConfig> = HashMap::new();
        project_step_configs.insert(String::from("my-step1"), ProjectStepConfig::default());
        project_step_configs.insert(String::from("my-step2"), ProjectStepConfig::default());

        // Build the service
        let mut service_builder =
            Service::from_config("my-service", &project_step_configs, &service_config)
                .expect("failed to create builder");
        service_builder
            .with_recipe(&project_step_configs, &recipe_config1)
            .expect("failed to use recipe");
        service_builder
            .with_recipe(&project_step_configs, &recipe_config2)
            .expect("failed to use recipe");

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
    fn service_builder_with_dependencies() {
        // Starting with a simple config
        let service_config = ServiceConfig {
            steps: vec![(
                String::from("my-step"),
                ServiceStepConfig {
                    depends_on: vec![String::from("my-service-dep")],
                    ..Default::default()
                },
            )]
            .into_iter()
            .collect(),
            ..Default::default()
        };

        // Project Step Configs
        let mut project_step_configs: HashMap<String, ProjectStepConfig> = HashMap::new();
        project_step_configs.insert(String::from("my-step"), ProjectStepConfig {
            depends_on: vec![String::from("my-step-dep")],
            ..Default::default()
        });

        // Build a service
        let service_builder =
            Service::from_config("my-service", &project_step_configs, &service_config)
                .expect("failed to create builder");
        let service = service_builder.build();

        // Get the step
        let service_step = service.steps.get("my-step").expect("failed to get step");

        assert_eq!(service.name, "my-service");
        assert_eq!(service_step.depends_on.len(), 2);
        assert!(service_step.depends_on.contains(&String::from("my-step:my-service-dep")));
        assert!(service_step.depends_on.contains(&String::from("my-step-dep:my-service")));
    }

    #[test]
    fn service_step_builder() {
        // Starting with a simple config
        let step_config = ServiceStepConfig {
            run: ScriptConfig::Boolean(true),
            ..Default::default()
        };
        let project_config = ProjectStepConfig::default();

        // Create the step builder
        let step_builder = ServiceStep::from_service_config(
            "my-step",
            "my-service",
            &project_config,
            &step_config,
        );

        // Build the ServiceStep
        let step = step_builder.build();

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
        let project_config = ProjectStepConfig::default();

        // Create the step builder
        let mut step_builder = ServiceStep::from_service_config(
            "my-step",
            "my-service",
            &project_config,
            &step_config,
        );

        // Apply the recipe
        step_builder.with_recipe(&recipe_config);

        // Build the ServiceStep
        let step = step_builder.build();

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
        let project_config = ProjectStepConfig::default();

        // Create the step builder
        let mut step_builder = ServiceStep::from_service_config(
            "my-step",
            "my-service",
            &project_config,
            &step_config,
        );

        // Apply the recipe
        step_builder.with_recipe(&recipe_config1);
        step_builder.with_recipe(&recipe_config2);

        // Build the ServiceStep
        let step = step_builder.build();

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
