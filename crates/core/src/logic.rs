#![allow(unused_imports, dead_code)]
use crate::domain::*;

//define actions
pub enum Action {
    //System
    Quit,
    NewProject,
    SaveProject,
    LoadProject,

    //Domain Actions
    File(FileAction),
    Schema(SchemaAction),
    Coding(CodingAction),
    View(ViewAction),
}

pub enum FileAction {

}

pub enum SchemaAction {

}

pub enum CodingAction {

}

pub enum ViewAction {

}
