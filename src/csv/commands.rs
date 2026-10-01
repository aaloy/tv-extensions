// (C) 2026 - Enzo Lombardi

//! Command ids. Turbo Vision leaves `[1000, 65535]` to applications.

use turbo_vision::core::command::CommandId;

/// Starts a new, blank document.
pub const CMD_NEW: CommandId = 1000;
/// Opens the file picker.
pub const CMD_OPEN: CommandId = 1001;
/// Saves to the document's current name.
pub const CMD_SAVE: CommandId = 1002;
/// Saves under a new name, asked for first.
pub const CMD_SAVE_AS: CommandId = 1003;
/// Closes the editor, asking first if there are unsaved changes.
pub const CMD_EXIT: CommandId = 1004;
/// Opens the edit-cell dialog for the selected cell.
pub const CMD_EDIT_CELL: CommandId = 1005;
/// Opens the rename-column dialog for the selected column.
pub const CMD_RENAME_COL: CommandId = 1006;
/// Inserts a row at the selection.
pub const CMD_ROW_INS: CommandId = 1007;
/// Deletes the selected row.
pub const CMD_ROW_DEL: CommandId = 1008;
/// Inserts a column at the selection.
pub const CMD_COL_INS: CommandId = 1009;
/// Deletes the selected column.
pub const CMD_COL_DEL: CommandId = 1010;
/// A dialog's OK button.
pub const CMD_DLG_OK: CommandId = 1011;
/// A dialog's Cancel button.
pub const CMD_DLG_CANCEL: CommandId = 1012;
/// The unsaved-changes dialog's Discard button.
pub const CMD_DISCARD: CommandId = 1013;
/// The Open dialog's list selection.
pub const CMD_OPEN_PICK: CommandId = 1014;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique_and_in_the_application_range() {
        let mut all = [
            CMD_NEW,
            CMD_OPEN,
            CMD_SAVE,
            CMD_SAVE_AS,
            CMD_EXIT,
            CMD_EDIT_CELL,
            CMD_RENAME_COL,
            CMD_ROW_INS,
            CMD_ROW_DEL,
            CMD_COL_INS,
            CMD_COL_DEL,
            CMD_DLG_OK,
            CMD_DLG_CANCEL,
            CMD_DISCARD,
            CMD_OPEN_PICK,
        ];
        assert!(all.iter().all(|c| *c >= 1000));
        all.sort_unstable();
        let n = all.len();
        let mut v = all.to_vec();
        v.dedup();
        assert_eq!(v.len(), n);
    }
}
