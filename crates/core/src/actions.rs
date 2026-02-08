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
            theme_id: Option<ThemeId>,
        },
        RenameCode { id: CodeDefId, name: String },
        UpdateCodeColor { id: CodeDefId, color: u8 },
        DeleteCode { id: CodeDefId },
        CreateTheme { name: String, color: u8 },
        RenameTheme { id: ThemeId, name: String },
        UpdateThemeColor { id: ThemeId, color: u8 },
        DeleteTheme { id: ThemeId },
        AssignCodeToTheme { code_id: CodeDefId, theme_id: ThemeId },
        RemoveCodeFromTheme { code_id: CodeDefId },
    }
    pub enum CodingAction {
        ApplyCode {
            code_def_id: CodeDefId,
            highlight: Highlight,
        },
        DeleteCode { id: QualCodeId },
        ReassignCode { id: QualCodeId, new_def_id: CodeDefId },
        EditHighlight { id: QualCodeId, new_highlight: Highlight },
    }
    #[derive(Debug)]
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
