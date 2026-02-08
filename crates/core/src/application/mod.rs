#![allow(dead_code, unused_variables)]
use crate::domain::*;
use crate::ports::*;
use crate::actions::*;
mod text_processing;
use text_processing::{split_into_blocks, extract_snippet, extract_context_before, extract_context_after};

use std::path::PathBuf;
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::sync::atomic::{AtomicBool, Ordering};
use anyhow::{Result, Context, bail};




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

    pub fn is_loaded(&self) -> bool {
        matches!(self.project, DataState::Loaded(_) | DataState::Modified(_))
    }

    /// Marks project as modified and bumps the save generation counter.
    pub fn mark_modified(&mut self) {
        self.project.mark_modified();
        self.save_generation += 1;
    }
}

/// Shared state for concurrent access between UI reads and async I/O writes.
pub type SharedState = Arc<RwLock<AppState>>;

/// Converts a Path to a String, returning an error if the path contains non-UTF-8 bytes.
fn path_to_string(path: &std::path::Path) -> Result<String> {
    path.to_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("File path contains non-UTF-8 characters: {:?}", path))
}

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
    /// Acquires a read lock, recovering from poison if a prior panic occurred.
    fn read_state(&self) -> RwLockReadGuard<'_, AppState> {
        self.state.read().unwrap_or_else(|e| e.into_inner())
    }

    /// Acquires a write lock, recovering from poison if a prior panic occurred.
    fn write_state(&self) -> RwLockWriteGuard<'_, AppState> {
        self.state.write().unwrap_or_else(|e| e.into_inner())
    }

    pub async fn new(state: SharedState, project_repo: P, file_loader: F, config_store: C) -> Result<Self> {
        let config = config_store.load_config()
            .await
            .unwrap_or_default();

        {
            let mut s = state.write().unwrap_or_else(|e| e.into_inner());
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

                let mut state = self.write_state();
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

                let mut state = self.write_state();
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
                    let state = self.read_state();
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

                result?;
                let mut state = self.write_state();
                if state.save_generation == captured_generation {
                    state.project.mark_saved();
                }
                Ok(ActionResult::Success)
            }
        }
    }

    async fn handle_file_action(&self, action: FileAction) -> Result<ActionResult> {
        match action {
            FileAction::ImportFile(path) => {
                let canonical = path.canonicalize()
                    .context("Failed to canonicalize file path")?;
                let canonical_str = path_to_string(&canonical)?;

                let name = canonical.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unnamed")
                    .to_string();

                let file_type = self.file_loader.detect_type(&canonical).await
                    .context("Failed to detect file type")?;

                let content = self.file_loader.read_file_content(&canonical).await
                    .context("Failed to read file content")?;

                let id = FileId::generate();
                let blocks = split_into_blocks(id, &content);

                let mut state = self.write_state();
                if !state.is_loaded() { bail!("No project loaded"); }
                if state.filemanager.has_path(&canonical_str) {
                    bail!(FileListError::DuplicatePath(canonical_str));
                }
                state.filemanager.add_file(id, name, canonical_str, file_type, blocks);
                state.mark_modified();
                Ok(ActionResult::FileImported(id))
            }
            FileAction::RemoveFile(id) => {
                let mut state = self.write_state();
                if !state.is_loaded() { bail!("No project loaded"); }
                let block_file_map = state.filemanager.build_block_file_map();
                state.codebook.remove_codes_for_file(id, &block_file_map);
                state.filemanager.remove_file(id)
                    .context("Failed to remove file")?;
                state.mark_modified();
                Ok(ActionResult::FileRemoved(id))
            }
            FileAction::ReloadFile(id, path) => {
                let canonical = path.canonicalize()
                    .context("Failed to canonicalize file path")?;
                let canonical_str = path_to_string(&canonical)?;

                let file_type = self.file_loader.detect_type(&canonical).await
                    .context("Failed to detect file type")?;

                let content = self.file_loader.read_file_content(&canonical).await
                    .context("Failed to read file content")?;

                let blocks = split_into_blocks(id, &content);

                let mut state = self.write_state();
                if !state.is_loaded() { bail!("No project loaded"); }
                let file = state.filemanager.file_mut(id)
                    .context("File not found for reload")?;
                file.reload(canonical_str, file_type, blocks);
                state.mark_modified();
                Ok(ActionResult::FileReloaded(id))
            }
        }
    }

    fn handle_schema_action(&self, action: SchemaAction) -> Result<ActionResult> {
        let mut state = self.write_state();
        if !state.is_loaded() { bail!("No project loaded"); }

        match action {
            SchemaAction::CreateCode { name, color, theme_id } => {
                let id = state.codebook.create_code_def(name, color, theme_id)
                    .context("Failed to create code definition")?;
                state.mark_modified();
                Ok(ActionResult::CodeCreated(id))
            }
            SchemaAction::RenameCode { id, name } => {
                let code = state.codebook.code_def_mut(id)
                    .ok_or(CodeBookError::CodeDefNotFound(id))
                    .context("Failed to rename code definition")?;
                code.set_name(name);
                state.mark_modified();
                Ok(ActionResult::Success)
            }
            SchemaAction::UpdateCodeColor { id, color } => {
                let code = state.codebook.code_def_mut(id)
                    .ok_or(CodeBookError::CodeDefNotFound(id))
                    .context("Failed to update code color")?;
                code.set_color(color);
                state.mark_modified();
                Ok(ActionResult::Success)
            }
            SchemaAction::DeleteCode { id } => {
                state.codebook.remove_code_def(id)
                    .context("Failed to delete code")?;
                state.mark_modified();
                Ok(ActionResult::Success)
            }
            SchemaAction::CreateTheme { name, color } => {
                let id = state.codebook.create_theme(name, color);
                state.mark_modified();
                Ok(ActionResult::ThemeCreated(id))
            }
            SchemaAction::RenameTheme { id, name } => {
                let theme = state.codebook.theme_mut(id)
                    .ok_or(CodeBookError::ThemeNotFound(id))
                    .context("Failed to rename theme")?;
                theme.set_name(name);
                state.mark_modified();
                Ok(ActionResult::Success)
            }
            SchemaAction::UpdateThemeColor { id, color } => {
                let theme = state.codebook.theme_mut(id)
                    .ok_or(CodeBookError::ThemeNotFound(id))
                    .context("Failed to update theme color")?;
                theme.set_color(color);
                state.mark_modified();
                Ok(ActionResult::Success)
            }
            SchemaAction::DeleteTheme { id } => {
                state.codebook.remove_theme(id)
                    .context("Failed to delete theme")?;
                state.mark_modified();
                Ok(ActionResult::Success)
            }
            SchemaAction::AssignCodeToTheme { code_id, theme_id } => {
                state.codebook.move_code_to_theme(code_id, theme_id)
                    .context("Failed to assign code to theme")?;
                state.mark_modified();
                Ok(ActionResult::Success)
            }
            SchemaAction::RemoveCodeFromTheme { code_id } => {
                state.codebook.remove_code_from_theme(code_id)
                    .context("Failed to remove code from theme")?;
                state.mark_modified();
                Ok(ActionResult::Success)
            }
        }
    }

    fn handle_coding_action(&self, action: CodingAction) -> Result<ActionResult> {
        let mut state = self.write_state();
        if !state.is_loaded() { bail!("No project loaded"); }

        match action {
            CodingAction::ApplyCode { code_def_id, highlight } => {
                state.codebook.code_def(code_def_id)
                    .ok_or(CodeBookError::CodeDefNotFound(code_def_id))
                    .context("Cannot apply code: definition not found")?;

                let snippet = extract_snippet(&state.filemanager, &highlight)
                    .context("Failed to extract snippet for applied code")?;
                let context_before = extract_context_before(&state.filemanager, &highlight);
                let context_after = extract_context_after(&state.filemanager, &highlight);

                let id = state.codebook.apply_code(code_def_id, highlight, snippet, context_before, context_after);
                state.mark_modified();
                Ok(ActionResult::CodeApplied(id))
            }
            CodingAction::DeleteCode { id } => {
                state.codebook.remove_qual_code(id)
                    .context("Failed to delete qual code")?;
                state.mark_modified();
                Ok(ActionResult::Success)
            }
            CodingAction::ReassignCode { id, new_def_id } => {
                state.codebook.code_def(new_def_id)
                    .ok_or(CodeBookError::CodeDefNotFound(new_def_id))
                    .context("Cannot reassign code: target definition not found")?;

                let qc = state.codebook.qual_code_mut(id)
                    .ok_or(CodeBookError::QualCodeNotFound(id))
                    .context("Cannot reassign code: qual code not found")?;
                qc.set_def_id(new_def_id);
                state.mark_modified();
                Ok(ActionResult::Success)
            }
            CodingAction::EditHighlight { id, new_highlight } => {
                let snippet = extract_snippet(&state.filemanager, &new_highlight)
                    .context("Failed to extract snippet for edited highlight")?;
                let context_before = extract_context_before(&state.filemanager, &new_highlight);
                let context_after = extract_context_after(&state.filemanager, &new_highlight);

                let qc = state.codebook.qual_code_mut(id)
                    .ok_or(CodeBookError::QualCodeNotFound(id))
                    .context("Cannot edit highlight: qual code not found")?;
                qc.update_highlight(new_highlight, snippet, context_before, context_after);
                state.mark_modified();
                Ok(ActionResult::Success)
            }
        }
    }
}

#[cfg(test)]
mod tests;
