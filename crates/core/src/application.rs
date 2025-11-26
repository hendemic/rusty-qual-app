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

///Handles all app actions and sends app commands to DB through defined interface
pub struct AppService<P: ProjectRepository, F: FileLoader, C: ConfigStore> {
    project: DataState<ProjectContext>,
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
    pub async fn new(project_repo: P, file_loader: F, config_store: C) -> Result<Self> {
        let codebook = CodeBook::new();
        let filemanager = FileList::new();
        let config = config_store.load_config()
            .await
            .unwrap_or_default() ;

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


    pub async fn handle_action(&mut self, action: Action) -> Result<ActionResult> {
        match action {
            Action::Project(a) => self.handle_project_action(a).await,
            Action::File(a) => self.handle_file_action(a).await,
            Action::Schema(a) => self.handle_schema_action(a),
            Action::Coding(a) => self.handle_coding_action(a),
            Action::Quit => Ok(ActionResult::Quit),
        }
    }

    async fn handle_project_action(&mut self, action: ProjectAction) -> Result<ActionResult> {
        match action {
            ProjectAction::NewProject { path, name } => {
                match self.project_repo.new_project(&path, name, &self.codebook, &self.filemanager).await {
                    Ok(project) => {
                        let ctx = ProjectContext::new(path, project);
                        self.project = DataState::Loaded(ctx);
                        Ok(ActionResult::Success)
                    }
                    Err(e) => {
                        if let DataState::Empty = self.project {
                            self.project = DataState::Error;
                        }
                        Err(e.into())
                    }
                }
            }
            ProjectAction::LoadProject(path) => {
                match self.project_repo.load_project(&path).await {
                    Ok((project, codebook, filemanager)) => {
                        let ctx = ProjectContext::new(path, project);
                        self.project = DataState::Loaded(ctx);
                        self.codebook = codebook;
                        self.filemanager = filemanager;
                        Ok(ActionResult::Success)
                    }
                    Err(e) => {
                        if let DataState::Empty = self.project {
                            self.project = DataState::Error;
                        }
                        Err(e.into())
                    }
                }
            }
            ProjectAction::SaveProject => {
                match &self.project {
                    DataState::Loaded(proj) | DataState::Modified(proj) => {
                        //project, codebook, and filemanager are cloned to pass a snapshot to infra to save
                        //app should maintain ownership of these, esp with this occuring async
                        match self.project_repo.save_project(&proj.path, proj.project.clone(), self.codebook.clone(), self.filemanager.clone()).await {
                            Ok(_) => Ok(ActionResult::Success),
                            Err(e) => Err(e.into())
                        }
                    }
                    _ => Err(ProjectError::Save("No project loaded".to_string()).into())
                }
            }
        }
    }

    async fn handle_file_action(&mut self, action: FileAction) -> Result<ActionResult> {
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
