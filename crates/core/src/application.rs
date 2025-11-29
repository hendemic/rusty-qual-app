#![allow(dead_code, unused_variables)]
use crate::domain::*;
use crate::ports::*;
use crate::actions::*;

use std::path::{ PathBuf };
use std::sync::{Arc, RwLock};
use anyhow::Result;
use serde::{Serialize, Deserialize};

/// **Shared app state wrapped for interior mutability**
///
/// Introduced to ensure AppState changes on I/O heavy operations don't block UI
/// Intention is that UI can read from shared state while async ops happen, and then
/// async I/O heavy function can lock for state changes in RAM.
pub type SharedState = Arc<RwLock<AppState>>;


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

/// **Core application state container**
///
/// Holds the current project context, codebook, file list, and configuration.
/// Wrapped in [`SharedState`] (`Arc<Mutex<>>`) to allow shared access across
/// the application while enabling safe mutation during brief, synchronous operations.
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


/// **Application controller that routes actions (in actions.rs) to their handlers**
///
/// Manages shared state via [`SharedState`] and coordinates async operations
/// with repositories without blocking UI access. All state mutations happen
/// through brief lock acquisitions around in-memory operations.
pub struct AppController <P: ProjectRepository, F: FileLoader, C: ConfigStore> {
    state: SharedState,
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
    pub async fn new(state: SharedState, project_repo: P, file_loader: F, config_store: C) -> Result<Self> {
        let config = config_store.load_config()
            .await
            .unwrap_or_default();

        {
            let mut s = state.write().unwrap();
            s.config = config;
        }

        Ok(Self {
            state,
            project_repo,
            file_loader,
            config_store,
        })
    }

    /// Receives Action and routes to appropriate handling function
    pub async fn handle_action(&self, action: Action) -> Result<ActionResult> {
        match action {
            Action::Project(a) => self.handle_project_action(a).await,
            Action::File(a) => self.handle_file_action(a).await,
            Action::Schema(a) => self.handle_schema_action(a),
            Action::Coding(a) => self.handle_coding_action(a),
            Action::Quit => Ok(ActionResult::Quit),
        }
    }

    async fn handle_project_action(&self, action: ProjectAction) -> Result<ActionResult> {
        match action {
            ProjectAction::NewProject { path, name } => {
                let result = self.project_repo.new_project(&path, name).await;

                let mut state = self.state.write().unwrap();
                match result {
                    Ok(project) => {
                        let ctx = ProjectContext::new(path, project);
                        state.project = DataState::Loaded(ctx);
                        Ok(ActionResult::Success)
                    }
                    Err(e) => {
                        if let DataState::Empty = state.project {
                            state.project = DataState::Error;
                        }
                        Err(e.into())
                    }
                }
            }
            ProjectAction::LoadProject(path) => {
                let result = self.project_repo.load_project(&path).await;

                let mut state = self.state.write().unwrap();
                match result {
                    Ok((project, codebook, filemanager)) => {
                        let ctx = ProjectContext::new(path, project);
                        state.project = DataState::Loaded(ctx);
                        state.codebook = codebook;
                        state.filemanager = filemanager;
                        Ok(ActionResult::Success)
                    }
                    Err(e) => {
                        if let DataState::Empty = state.project {
                            state.project = DataState::Error;
                        }
                        Err(e.into())
                    }
                }
            }
            ProjectAction::SaveProject => {
                let save_data = {
                    let state = self.state.read().unwrap();
                    match &state.project {
                        DataState::Loaded(proj) | DataState::Modified(proj) => {
                            Some((proj.path.clone(), proj.project.clone(), state.codebook.clone(), state.filemanager.clone()))
                        }
                        _ => None
                    }
                };

                match save_data {
                    Some((path, project, codebook, filemanager)) => {
                        match self.project_repo.save_project(&path, project, codebook, filemanager).await {
                            Ok(_) => Ok(ActionResult::Success),
                            Err(e) => Err(e.into())
                        }
                    }
                    None => Err(ProjectError::Save("No project loaded".to_string()).into())
                }
            }
        }
    }

    async fn handle_file_action(&self, action: FileAction) -> Result<ActionResult> {
        match action {
            FileAction::AddFile(path) => {
                todo!("build out filing adding")
            }
            FileAction::LoadFile(id) => {
                todo!("build out opening")
            }
        }
    }

    fn handle_schema_action(&self, action: SchemaAction) -> Result<ActionResult> {
        match action {
            SchemaAction::CreateCode { name, color } => {
                todo!("build out code creation")
            }
        }
    }

    fn handle_coding_action(&self, action: CodingAction) -> Result<ActionResult> {
        match action {
            CodingAction::ApplyCode { code_def_id, highlight, snippet } => {
                todo!("build out code creation")
            }
        }
    }
}
