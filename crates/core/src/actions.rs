    use crate::domain::*;
    use std::path::PathBuf;

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
