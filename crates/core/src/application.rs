#![allow(dead_code, unused_variables)]
use crate::domain::*;
use crate::ports::*;
use crate::actions::*;

use std::path::{ PathBuf };
use anyhow::Result;
use serde::{Serialize, Deserialize};


// TODO this is a placeholder. Need to define configuration params
// and how I handle theming (which is probably not in the first front end proof of concept)
#[derive(Serialize, Deserialize)]
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

pub struct AppState {
    project: DataState<ProjectContext>,
    codebook: CodeBook,
    filemanager: FileList,
    config: AppConfig,
}

impl AppState {
    pub fn new(project: DataState<ProjectContext>, config: AppConfig) -> Self {
        let codebook = CodeBook::new();
        let filemanager = FileList::new();
        AppState { project, codebook, filemanager, config }
    }
}


///Handles all app actions and sends app commands to DB through defined interface
pub struct AppController <P: ProjectRepository, F: FileLoader, C: ConfigStore> {
    project_repo: P,
    file_loader: F,
    config_store: C,
}

impl<P, F, C> AppController <P, F, C>
where
    P: ProjectRepository,
    F: FileLoader,
    C: ConfigStore,
{
    pub async fn new(project_repo: P, file_loader: F, config_store: C) -> Result<Self> {
        let config = config_store.load_config()
            .await
            .unwrap_or_default() ;

        Ok(Self {
            project_repo,
            file_loader,
            config_store,
        })
    }

    pub async fn handle_action(&self, app_state: &mut AppState, action: Action) -> Result<ActionResult> {
        match action {
            Action::Project(a) => self.handle_project_action(app_state, a).await,
            Action::File(a) => self.handle_file_action(app_state, a).await,
            Action::Schema(a) => self.handle_schema_action(app_state, a),
            Action::Coding(a) => self.handle_coding_action(app_state, a),
            Action::Quit => Ok(ActionResult::Quit),
        }
    }

    async fn handle_project_action(&self, app_state: &mut AppState, action: ProjectAction) -> Result<ActionResult> {
        match action {
            ProjectAction::NewProject { path, name } => {
                match self.project_repo.new_project(&path, name).await {
                    Ok(project) => {
                        let ctx = ProjectContext::new(path, project);
                        app_state.project = DataState::Loaded(ctx);
                        Ok(ActionResult::Success)
                    }
                    Err(e) => {
                        if let DataState::Empty = app_state.project {
                            app_state.project = DataState::Error;
                        }
                        Err(e.into())
                    }
                }
            }
            ProjectAction::LoadProject(path) => {
                match self.project_repo.load_project(&path).await {
                    Ok((project, codebook, filemanager)) => {
                        let ctx = ProjectContext::new(path, project);
                        app_state.project = DataState::Loaded(ctx);
                        app_state.codebook = codebook;
                        app_state.filemanager = filemanager;
                        Ok(ActionResult::Success)
                    }
                    Err(e) => {
                        if let DataState::Empty = app_state.project {
                            app_state.project = DataState::Error;
                        }
                        Err(e.into())
                    }
                }
            }
            ProjectAction::SaveProject => {
                match &app_state.project {
                    DataState::Loaded(proj) | DataState::Modified(proj) => {
                        //project, codebook, and filemanager are cloned to pass a snapshot to infra to save
                        //app should maintain ownership of these, esp with this occuring async
                        match self.project_repo.save_project(&proj.path, proj.project.clone(), app_state.codebook.clone(), app_state.filemanager.clone()).await {
                            Ok(_) => Ok(ActionResult::Success),
                            Err(e) => Err(e.into())
                        }
                    }
                    _ => Err(ProjectError::Save("No project loaded".to_string()).into())
                }
            }
        }
    }

    async fn handle_file_action(&self, app_state: &mut AppState, action: FileAction) -> Result<ActionResult> {
        match action {
            FileAction::AddFile(path) => {
                todo!("build out filing adding")
            }
            FileAction::LoadFile(id) => {
                todo!("build out opening")
            }
        }
    }

    fn handle_schema_action(&self, app_state: &mut AppState, action: SchemaAction) -> Result<ActionResult> {
        match action {
            SchemaAction::CreateCode { name, color } => {
                todo!("build out code creation")
            }
        }
    }

    fn handle_coding_action(&self, app_state: &mut AppState, action: CodingAction) -> Result<ActionResult> {
        match action {
            CodingAction::ApplyCode { code_def_id, highlight, snippet } => {
                todo!("build out code creation")
            }
        }
    }
}