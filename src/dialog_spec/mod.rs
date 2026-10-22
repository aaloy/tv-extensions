// (C) 2026 - Antoni Aloy

//! Dialogs designed with the dialog designer, saved as JSON.

mod runtime;
mod spec;

pub use runtime::{ActionContext, Answer, Errors, Flow, JsonDialog};
pub use spec::{
    Align, ColumnSpec, Control, DialogSpec, Item, Kind, MAX_CLUSTER_ITEMS, PageSpec, SpecError,
    VERSION, ValidatorSpec,
};
