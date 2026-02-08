    use crate::domain::*;
    use std::path::PathBuf;

    //define actions
    pub enum Action {
        Quit,
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
        ImportFile(PathBuf),
        RemoveFile(FileId),
        ReloadFile(FileId, PathBuf),
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
        SaveInProgress,
        ThemeCreated(ThemeId),
        CodeCreated(CodeDefId),
        FileImported(FileId),
        FileRemoved(FileId),
        FileReloaded(FileId),
        CodeApplied(QualCodeId),
    }
