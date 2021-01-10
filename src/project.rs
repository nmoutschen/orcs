use crate::{
    config::{ProjectConfig, RecipeConfig, ServiceConfig},
    utils::load_config,
    Error, Result, Service,
};
use git2::Repository;
use std::cell::Cell;
use std::collections::HashMap;
use std::fs::read_dir;
use std::path::{Path, PathBuf};
use std::rc::Rc;

const PROJECT_CONFIG_FILENAME: &'static str = "orcs.toml";
const SERVICE_CONFIG_FILENAME: &'static str = "orcs.toml";
const SERVICE_FOLDER: &'static str = "srv";
const RECIPE_FOLDER: &'static str = "rcp";

#[derive(Default)]
/// Orcs Project
pub struct Project {
    /// Root folder for the project
    path: PathBuf,

    /// Configuration file for the project
    config: ProjectConfig,

    /// Loaded services for the project.
    ///
    /// Users shouldn't interact with this directly (thus this is set to
    /// private), but use the `get_service()` and `get_all_services()` functions
    /// to retrieve one or multiple services.
    services: Cell<HashMap<String, Rc<Service>>>,

    /// Flag if we've already loaded all services or not.
    ///
    /// Since we could have calls to `get_service()` before a
    /// `get_all_services()` call, having data in the service HashMap is not a
    /// good indicator if we have all services loaded already. Therefore, we
    /// need a flag to load this explicitely.
    services_all_loaded: Cell<bool>,

    /// Loaded recipes for the project.
    recipes: Cell<HashMap<String, Rc<RecipeConfig>>>,
}

impl Project {
    /// Load the project from a path
    ///
    /// This will read the project configuration file in the folder and return
    /// a Project instance if it was able to load the project correctly.
    pub fn from_path<P>(path: P) -> Result<Self>
    where
        P: Into<PathBuf>,
    {
        let path = path.into();

        // Loading configuration
        let config = load_config(path.join(PROJECT_CONFIG_FILENAME))?;

        // Return the project
        let project = Self {
            path: path.clone(),
            config,

            ..Default::default()
        };

        // Validate the project
        project.validate()?;

        Ok(project)
    }

    // TODO
    // /// Create a new project from scratch
    // pub fn create<P>(path: P) -> Result<Self>
    // where
    //     P: Into<Pathbuf>,
    // {

    // }

    /// Check if the project is correct
    fn validate(&self) -> Result<()> {
        // TODO:
        // * Check for reserved names in steps
        // * Check that the step dependency graph is acyclic and all values
        //   exist

        // Check if the project is a repository
        Repository::open(&self.path).map_err(|source| Error::ProjectIsNotGitRepo {
            path: self.path.clone(),
            source,
        })?;

        Ok(())
    }

    /// Get a service from its name
    ///
    /// If the service was already loaded before, return it from the Project's
    /// internal store, otherwise fetch it.
    pub fn get_service(&self, service_name: &str) -> Result<Rc<Service>> {
        let mut services = self.services.take();
        // Only load the service if we haven't loaded it already
        if !services.contains_key(service_name) {
            let service = Rc::new(self.load_service(service_name)?);

            services.insert(service_name.to_string(), service);
        }

        let service = services
            .get(service_name)
            .expect("failed to get service")
            .clone();

        // Set back the HashMap in the Cell
        self.services.set(services);

        Ok(service)
    }

    /// Return all services for a given project
    ///
    /// The first time this method is called, it will scan the project folder
    /// for all projects and save it into the Project's internal state (thus
    /// the need to pass a mutable reference).
    pub fn get_all_services(&self) -> Result<HashMap<String, Rc<Service>>> {
        let mut services = self.services.take();
        if !self.services_all_loaded.get() {
            // Load all services
            services.extend(self.scan_services(self.path.join(SERVICE_FOLDER))?);

            self.services_all_loaded.set(true);
        }

        // Return all services
        //
        // This clones the `HashMap` and `Rc`s, but not the internal `Service`
        // structs.
        let retval = services.clone();

        // Set back the HashMap in the Cell
        self.services.set(services);

        Ok(retval)
    }

    // /// Get a recipe from its name
    // ///
    // /// If the recipe was already loaded before, return it from the Project's
    // /// internal store, otherwise fetch it.
    // fn get_recipe(&self, recipe_name: &str) -> Result<Rc<RecipeConfig>> {
    //     let mut recipes = self.recipes.get_mut();
    //     // Only load the recipe if it wasn't loaded previously
    //     if !recipes.contains_key(recipe_name) {
    //         let recipe = self.load_recipe_config(recipe_name)?;
    //         recipes.insert(recipe_name.to_string(), recipe);
    //     }

    //     Ok(recipes
    //         .get(recipe_name)
    //         .expect("failed to get recipe")
    //         .clone())
    // }

    /// Retrieve multiple recipes at once
    ///
    /// This will return the recipes in the same order as the names provided.
    fn get_recipes(&self, recipe_names: &[String]) -> Result<Vec<Rc<RecipeConfig>>> {
        let mut recipes = self.recipes.take();
        // First loop to perform mutable operations (loading and storing the
        // recipes that we haven't scanned yet).
        for recipe_name in recipe_names {
            if !recipes.contains_key(recipe_name) {
                let recipe = self.load_recipe_config(recipe_name)?;
                recipes.insert(recipe_name.to_string(), Rc::new(recipe));
            }
        }

        // Second loop to retrieve the recipes requested
        let mut req_recipes: Vec<Rc<RecipeConfig>> = Default::default();
        for recipe_name in recipe_names {
            req_recipes.push(
                recipes
                    .get(recipe_name)
                    .expect("failed to get recipe")
                    .clone(),
            );
        }

        // Set back the HashMap in the Cell
        self.recipes.set(recipes);

        Ok(req_recipes)
    }

    /// Try to find services in the given folder
    ///
    /// This will recursively scan all folders in a given `dir` to try to find
    /// all services and will return a HashMap with all values.
    fn scan_services<P>(&self, dir: P) -> Result<HashMap<String, Rc<Service>>>
    where
        P: AsRef<Path>,
    {
        let mut services: HashMap<String, Rc<Service>> = Default::default();

        let dir = dir.as_ref();

        // TODO: handle errors
        // TODO: ignore some paths
        for entry in read_dir(dir).unwrap() {
            let path = entry.unwrap().path();

            if !path.is_dir() {
                continue;
            } else if path.join(SERVICE_CONFIG_FILENAME).is_file() {
                // We found a service
                let service_name = self.get_service_name(&path);
                let service = self.load_service(&service_name)?;
                services.insert(service_name, Rc::new(service));
            } else {
                // This is a folder, but there's no service configuration file,
                // therefore we should scan it too
                services.extend(self.scan_services(&path)?)
            }
        }

        Ok(services)
    }

    /// Internal method to load a service
    fn load_service(&self, service_name: &str) -> Result<Service> {
        // Load the config
        let service_config: ServiceConfig = load_config(
            self.path
                .join(SERVICE_FOLDER)
                .join(service_name)
                .join(SERVICE_CONFIG_FILENAME),
        )?;

        // Create a ServiceBuilder
        let mut service = Service::from_config(service_name, &service_config);

        // Parse all recipes in the service config
        let recipes = self.get_recipes(&service_config.recipes)?;
        // Remark: We need to invert the recipe order to process them in the
        // right order. In the configuration file, the latest recipe takes
        // precedence over the previous ones. However,
        // `ServiceBuilder::with_recipe` works the other way around for
        // simplicity's sake.
        for recipe in recipes.iter().rev() {
            service.with_recipe(recipe);
        }

        Ok(service.build())
    }

    /// Load a recipe configuration file
    fn load_recipe_config(&self, recipe_name: &str) -> Result<RecipeConfig> {
        load_config(
            self.path
                .join(RECIPE_FOLDER)
                .join(format!("{}.toml", recipe_name)),
        )
    }

    /// Transform a service path into a canonical name representation
    fn get_service_name<P>(&self, path: P) -> String
    where
        P: AsRef<Path>,
    {
        path.as_ref()
            .strip_prefix(self.path.join(SERVICE_FOLDER))
            .unwrap()
            .to_string_lossy()
            .replace("\\", "/")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{create_dir_all, File};
    use std::io::prelude::*;
    use tempfile::tempdir;

    /// Create a project named 'my-project' with a step 'my-step'
    fn create_project() -> tempfile::TempDir {
        let project_name = "my-project";
        let step_name = "my-step";

        // Create a temporary project folder
        let project_dir = tempdir().expect("failed to create a temporary project folder");
        let folder = project_dir.path();

        // Initialize a git repository
        Repository::init(&folder).expect("failed to create a git repository");

        // Create a project config file
        let mut cfg_file = File::create(folder.join(PROJECT_CONFIG_FILENAME))
            .expect("failed to create the project config file");
        let cfg_data = &format!(
            "
        name = \"{project_name}\"

        [steps.{step_name}]
        ",
            project_name = project_name,
            step_name = step_name
        );
        cfg_file
            .write_all(cfg_data.as_bytes())
            .expect("failed to write project config file");

        project_dir
    }

    fn create_service<P>(path: P, name: &str)
    where
        P: AsRef<Path>,
    {
        // Create service folder
        let service_path = path.as_ref().join(SERVICE_FOLDER).join(name);
        create_dir_all(&service_path).expect("unable to create service folder");

        // Create service config file
        let mut config_file = File::create(service_path.join(SERVICE_CONFIG_FILENAME))
            .expect("unable to create service config file");
        let config_data = "
        [steps.my-step]
        ";
        config_file
            .write_all(config_data.as_bytes())
            .expect("unable to write service config file");
    }

    // fn create_recipe<P>(path: P, name: &str)
    // where
    //     P: AsRef<Path>,
    // {
    //     // Create service folder
    //     let recipe_path = path.as_ref().join(RECIPE_FOLDER);
    //     create_dir_all(&recipe_path).expect("unable to create recipe folder");

    //     // Create service config file
    //     let mut config_file = File::create(recipe_path.join(format!("{}.toml", name)))
    //         .expect("unable to create recipe config file");
    //     let config_data = "
    //     [steps.my-step]
    //     check = \"my-check-script\"
    //     run = \"my-run-script\"
    //     ";
    //     config_file
    //         .write_all(config_data.as_bytes())
    //         .expect("unable to write recipe config file");
    // }

    #[test]
    fn load_from_path() {
        // Create the project
        let project_name = "my-project";
        let step_name = "my-step";
        let project_dir = create_project();
        let folder = project_dir.path();

        // Load the project
        // This should return an Ok(_) value.
        let project = Project::from_path(folder).expect("failed to load the project");

        // Check if all values are correct
        assert_eq!(project.path, folder);
        assert_eq!(project.config.name, project_name);
        assert!(project.config.steps.contains_key(step_name));
    }

    #[test]
    fn load_from_path_service() {
        // Create the project
        let project_name = "my-project";
        let step_name = "my-step";
        let project_dir = create_project();
        let folder = project_dir.path();

        // Create a service
        create_service(&folder, "my-service");

        // Load the project
        // This should return an Ok(_) value.
        let project = Project::from_path(folder).expect("failed to load the project");

        // Load a service
        let service = project
            .get_service("my-service")
            .expect("failed to get service");

        // Check if all values are correct
        assert_eq!(project.path, folder);
        assert_eq!(project.config.name, project_name);
        assert!(project.config.steps.contains_key(step_name));
        assert_eq!(service.name, "my-service");
        let service_step = service
            .get_step("my-step")
            .expect("failed to get service step");
        assert_eq!(service_step.name, "my-step:my-service");

        let services = project.services.take();
        assert!(services.contains_key("my-service"));
    }

    #[test]
    fn get_service_name() {
        // Create a temporary project folder
        let project_dir = tempdir().expect("failed to create a temporary project folder");
        let folder = project_dir.path();
        let project = Project {
            path: folder.to_path_buf(),
            ..Default::default()
        };
        let value = "a/b/c";

        // Retrieve the service name
        let result = project.get_service_name(folder.join(SERVICE_FOLDER).join(value));

        // Compare the value
        assert_eq!(result, value);
    }
}
