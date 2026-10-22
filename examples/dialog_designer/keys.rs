// (C) 2026 - Antoni Aloy

//! The designer's keys. A Mac keyboard has no Insert, Delete (its key sends
//! Backspace), Page Up/Down or usable F-keys, so there each of those has a
//! Ctrl+letter substitute. `DIALOG_DESIGNER_KEYS=mac|pc` overrides the guess.

/// The key shown and bound for every action that has a PC/Mac difference.
#[derive(Debug, PartialEq, Eq)]
pub struct Keys {
    pub open: &'static str,
    pub save: &'static str,
    pub exit: &'static str,
    pub insert: &'static str,
    pub delete: &'static str,
    pub earlier: &'static str,
    pub later: &'static str,
    pub move_to: &'static str,
    pub test: &'static str,
    pub json: &'static str,
    pub order: &'static str,
    pub page_back: &'static str,
    pub page_forward: &'static str,
}

pub const PC: Keys = Keys {
    open: "F3",
    save: "F2",
    exit: "Alt+X",
    insert: "Ins",
    delete: "Del",
    earlier: "F7",
    later: "F8",
    move_to: "F6",
    test: "F9",
    json: "F4",
    order: "Ctrl+O",
    page_back: "Ctrl+PgUp",
    page_forward: "Ctrl+PgDn",
};

pub const MAC: Keys = Keys {
    open: "Ctrl+O",
    save: "Ctrl+S",
    exit: "Ctrl+Q",
    insert: "Ctrl+A",
    delete: "Backspace",
    earlier: "Ctrl+U",
    later: "Ctrl+L",
    move_to: "Ctrl+P",
    test: "Ctrl+E",
    json: "Ctrl+K",
    order: "Ctrl+T",
    page_back: "Ctrl+W",
    page_forward: "Ctrl+V",
};

/// Which table to use: `choice` ("mac" or "pc") wins, else the platform.
pub fn pick(choice: Option<&str>, mac: bool) -> &'static Keys {
    match choice.map(str::to_ascii_lowercase).as_deref() {
        Some("mac") => &MAC,
        Some("pc") => &PC,
        _ if mac => &MAC,
        _ => &PC,
    }
}

/// The keys for this machine.
pub fn current() -> &'static Keys {
    pick(
        std::env::var("DIALOG_DESIGNER_KEYS").ok().as_deref(),
        cfg!(target_os = "macos"),
    )
}

/// The status line text for a key: `Ctrl+S` is shown `Ctrl-S`.
pub fn label(chord: &str) -> String {
    chord.replace('+', "-")
}

/// The Help ▸ Keys text with this keyboard's keys.
pub fn help(k: &Keys) -> String {
    format!(
        "\
Mouse
  Click            select a control
  Drag             move it
  Drag the yellow  resize it (or the dialog)
    corner
  Drag the frame   move the dialog about the desktop
  Double click     properties
  Inspector tree   click / arrows select a control, Enter edits it,
                   drag a control onto a container to move it
  Right click      insert a control

Keys
  Tab / Shift+Tab  next / previous control
  Esc              select the container
  Arrows           move one cell (the dialog: move it)
  Shift/Ctrl+Arrow resize one cell
  Enter            properties
  {insert:<16} insert a control
  {delete:<16} delete
  Ctrl+D           duplicate
  {tab_order:<16} earlier / later in tab order
  {move_to:<16} move to another container
  {pages:<16} previous / next tab page
  {order:<16} show tab order
  Ctrl+Z / Ctrl+Y  undo / redo
  {test:<16} test the dialog
  {json:<16} show the JSON
  Ctrl+G           save the code to use it
  {files:<16} save / open
  {exit:<16} exit",
        insert = k.insert,
        delete = k.delete,
        tab_order = format!("{} / {}", k.earlier, k.later),
        move_to = k.move_to,
        pages = format!("{} / {}", k.page_back, k.page_forward),
        order = k.order,
        test = k.test,
        json = k.json,
        files = format!("{} / {}", k.save, k.open),
        exit = k.exit,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_platform_picks_the_table() {
        assert_eq!(pick(None, true), &MAC);
        assert_eq!(pick(None, false), &PC);
    }

    #[test]
    fn the_override_beats_the_platform() {
        assert_eq!(pick(Some("PC"), true), &PC);
        assert_eq!(pick(Some("mac"), false), &MAC);
        assert_eq!(pick(Some("other"), true), &MAC);
    }

    #[test]
    fn the_mac_table_has_no_pc_only_keys() {
        for k in [
            MAC.open, MAC.save, MAC.exit, MAC.insert, MAC.delete, MAC.earlier, MAC.later,
            MAC.move_to, MAC.test, MAC.json, MAC.order, MAC.page_back, MAC.page_forward,
        ] {
            assert!(
                !k.starts_with('F') && k != "Ins" && !k.contains("Pg") && !k.contains("Alt"),
                "{k}"
            );
        }
    }

    #[test]
    fn the_mac_keys_do_not_clash() {
        let mut all = vec![
            MAC.open, MAC.save, MAC.exit, MAC.insert, MAC.earlier, MAC.later, MAC.move_to,
            MAC.test, MAC.json, MAC.order, MAC.page_back, MAC.page_forward,
            // the ones both keyboards share
            "Ctrl+N", "Ctrl+Z", "Ctrl+Y", "Ctrl+D", "Ctrl+G",
        ];
        let n = all.len();
        all.sort_unstable();
        all.dedup();
        assert_eq!(all.len(), n);
    }

    #[test]
    fn the_help_names_this_keyboards_keys() {
        assert!(help(&MAC).contains("Ctrl+A"));
        assert!(!help(&MAC).contains("F9"));
        assert!(help(&PC).contains("F9"));
    }
}
