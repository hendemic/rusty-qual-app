#![allow(dead_code, unused_variables)]
use crate::domain::*;
use crate::ports::*;
use crate::actions::*;

use std::path::{ PathBuf };
use std::sync::{Arc, RwLock};
use std::sync::atomic::{AtomicBool, Ordering};
use anyhow::{Result, Context};




/// Resets `is_saving` on Drop so the flag is always cleared.
struct SavingGuard<'a>(&'a AtomicBool);

impl Drop for SavingGuard<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
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


/// Core application state. Wrapped in [`SharedState`] for shared access
/// across async boundaries.
pub struct AppState {
    project: DataState<ProjectContext>,
    codebook: CodeBook,
    filemanager: FileList,
    config: AppConfig,
    /// Tracks mutations so save can detect if state changed during I/O.
    save_generation: u64,
}


impl AppState {
    pub fn new(project: DataState<ProjectContext>, config: AppConfig) -> Self {
        let codebook = CodeBook::new();
        let filemanager = FileList::new();
        AppState { project, codebook, filemanager, config, save_generation: 0 }
    }

    /// Marks project as modified and bumps the save generation counter.
    pub fn mark_modified(&mut self) {
        self.project.mark_modified();
        self.save_generation += 1;
    }
}

/// Shared state for concurrent access between UI reads and async I/O writes.
pub type SharedState = Arc<RwLock<AppState>>;

/// Routes [`Action`] variants to their handlers, coordinating state and I/O.
pub struct AppController <P: ProjectRepository, F: FileHandler, C: ConfigStore> {
    state: SharedState,
    project_repo: P,
    file_loader: F,
    config_store: C,
    is_saving: AtomicBool,
}

impl<P, F, C> AppController <P, F, C>
where
    P: ProjectRepository,
    F: FileHandler,
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
            is_saving: AtomicBool::new(false),
        })
    }

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
                        Err(e).context("Failed to create new project")
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
                        Err(e).context("Failed to load project")
                    }
                }
            }
            ProjectAction::SaveProject => {
                if self.is_saving.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_err() {
                    return Ok(ActionResult::SaveInProgress);
                }
                let _guard = SavingGuard(&self.is_saving);

                let (save_data, captured_generation) = {
                    let state = self.state.read().unwrap();
                    let data = match &state.project {
                        DataState::Loaded(proj) | DataState::Modified(proj) => {
                            Some((proj.path.clone(), proj.project.clone(), state.codebook.clone(), state.filemanager.clone()))
                        }
                        _ => None
                    };
                    (data, state.save_generation)
                };

                let result = match save_data {
                    Some((path, project, codebook, filemanager)) => {
                        self.project_repo.save_project(&path, project, codebook, filemanager).await
                    }
                    None => {
                        return Err(ProjectError::Save("No project loaded".to_string()).into());
                    }
                };

                match result {
                    Ok(_) => {
                        let mut state = self.state.write().unwrap();
                        if state.save_generation == captured_generation {
                            state.project.mark_saved();
                        }
                        Ok(ActionResult::Success)
                    }
                    Err(e) => Err(e)
                }
            }
        }
    }

    async fn handle_file_action(&self, action: FileAction) -> Result<ActionResult> {
        match action {
            FileAction::AddFile(path) => {
                todo!("build out file adding")
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

#[cfg(test)]
mod tests;
