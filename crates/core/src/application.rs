#![allow(dead_code, unused_variables)]
use crate::domain::*;
use crate::ports::*;
use std::path::{ PathBuf };
use anyhow::Result;


//define actions
pub enum Action {
    //system
    Quit,

    //domain actions
    Project(ProjectAction),
    File(FileAction),
    Schema(SchemaAction),
    Coding(CodingAction),
}

pub enum ProjectAction {
    NewProject{
        path: PathBuf,
        name: String,
    },
    SaveProject,
    LoadProject(PathBuf),
}

pub enum FileAction {
    AddFile(PathBuf),
    OpenFile(FileId),
    //FindFile(FileId) save this for future. noting here because losing file ref is important MVP handling
}

pub enum SchemaAction {
    CreateCode{
        name: String,
        color: u8,
    },
}

pub enum CodingAction {
    ApplyCode {
        code_def_id: CodeDefId,
        highlight: Highlight,
        snippet: String,
    },
}

pub enum ActionResult {
    Quit,
    Success,
    ThemeCreated(ThemeId),
    CodeCreated(CodeDefId),
    FileAdded(FileId),
    CodeApplied(QualCodeId),
}

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

pub struct LoadedProject {
    project: QualProject,
    path: PathBuf,
    root: PathBuf
}

impl LoadedProject {
    pub fn new(path: PathBuf, project: QualProject) -> Result<Self> {
        let root = path.parent()
            .ok_or(anyhow::anyhow!("Project path must have parent directory"))?
            .to_path_buf();

        Ok(LoadedProject {
            project,
            path,
            root,
        })
    }
}

///Handles all app actions and sends app commands to DB through defined interface
pub struct AppService<P: ProjectRepository, F: FileLoader, C: ConfigStore> {
    project: DataState<LoadedProject, ProjectError>,
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
                let project = self.project_repo.new_project(&path, name, &self.codebook, &self.filemanager)?;
                self.project = DataState::Loaded(LoadedProject::new(path, project)?);
                Ok(ActionResult::Success)
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
