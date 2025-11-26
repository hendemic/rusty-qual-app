#![allow(dead_code, unused_variables)]
use crate::domain::*;
use crate::ports::*;
use crate::actions::*;

use std::path::{ PathBuf };
use anyhow::Result;



pub struct AppConfig {
    pub theme: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        AppConfig {
            theme: "dark".to_string(),
        }
    }
}

pub struct ProjectContext {
    project: QualProject,
    path: PathBuf,
    root: PathBuf
}

impl ProjectContext {
    pub fn new(path: PathBuf, project: QualProject) -> Self {
        let root = path.parent()
            .expect("Project path must have parent")
            .to_path_buf();

        ProjectContext { project, path, root }
    }
}

///Handles all app actions and sends app commands to DB through defined interface
pub struct AppService<P: ProjectRepository, F: FileLoader, C: ConfigStore> {
    project: DataState<ProjectContext, ProjectError>,
    codebook: CodeBook,
    filemanager: FileList,
    config: AppConfig,

    //ports for infrastructure
    project_repo: P,
    file_loader: F,
    config_store: C,
}

impl<P, F, C> AppService<P, F, C>
where
    P: ProjectRepository,
    F: FileLoader,
    C: ConfigStore,
{
    pub fn new(project_repo: P, file_loader: F, config_store: C) -> Result<Self> {
        let codebook = CodeBook::new();
        let filemanager = FileList::new();
        let config = config_store.load_config().unwrap_or_default();

        Ok(Self {
            project: DataState::Empty,
            codebook,
            filemanager,
            config,
            project_repo,
            file_loader,
            config_store,
        })
    }

    // TODO: Look into async for handle_action, which could freeze the UI
    // In general, its much more a technical lift, but all of the core application should be async
    // once a proof of concept is together.
    pub fn handle_action(&mut self, action: Action) -> Result<ActionResult> {
        match action {
            Action::Project(a) => self.handle_project_action(a),
            Action::File(a) => self.handle_file_action(a),
            Action::Schema(a) => self.handle_schema_action(a),
            Action::Coding(a) => self.handle_coding_action(a),
            Action::Quit => Ok(ActionResult::Quit),
        }
    }

    fn handle_project_action(&mut self, action: ProjectAction) -> Result<ActionResult> {
        match action {
            ProjectAction::NewProject { path, name } => {
                match self.project_repo.new_project(&path, name, &self.codebook, &self.filemanager) {
                    Ok(project) => {
                        let ctx = ProjectContext::new(path, project);
                        self.project = DataState::Loaded(ctx);
                        Ok(ActionResult::Success)
                    }
                    Err(e) => {
                        self.project = DataState::Error(ProjectError::New);
                        Err(e)
                    }
                }
            }

            ProjectAction::LoadProject(path) => {
                todo!("build out loading")
            }
            ProjectAction::SaveProject => {
                todo!("build out saving")
            }
        }
    }
    fn handle_file_action(&mut self, action: FileAction) -> Result<ActionResult> {
        match action {
            FileAction::AddFile(path) => {
                todo!("build out filing adding")
            },
            FileAction::OpenFile(id) => {
                todo!("build out opening")
            }
        }
    }

    fn handle_schema_action(&mut self, action: SchemaAction) -> Result<ActionResult> {
        match action {
            SchemaAction::CreateCode { name, color } => {
                todo!("build out code creation")
            }
        }
    }

    fn handle_coding_action(&mut self, action: CodingAction) -> Result<ActionResult> {
        match action {
            CodingAction::ApplyCode { code_def_id, highlight, snippet } => {
                todo!("build out code creation")
            }
        }
    }
}
