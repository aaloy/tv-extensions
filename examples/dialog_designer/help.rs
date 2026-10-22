// (C) 2026 - Antoni Aloy

//! Contextual help: what the selected control is for and what its
//! properties mean.

use crate::keys::Keys;
use crate::model::KINDS;

/// The help on a control type (the JSON `type`).
#[allow(clippy::too_many_lines)] // one arm of prose per control type
pub fn control(type_name: &str) -> &'static str {
    match type_name {
        "label" => {
            "A piece of text, usually the caption of the control next to it.\n\
             \n\
             text   the caption; ~X~ makes X a hot key\n\
             link   the control the hot key moves to\n\
             color  one of 16 colours"
        }
        "static_text" => {
            "Text that wraps over several lines, for explanations.\n\
             \n\
             text   the words; they wrap to the control's width"
        }
        "line" => {
            "A rule or a rectangle for dividing the dialog up.\n\
             \n\
             One row tall it is a horizontal rule, one column wide a vertical\n\
             one, larger a rectangle with corners.\n\
             \n\
             style  single, double, heavy, round or block\n\
             color  one of 16 colours"
        }
        "input" => {
            "A line to type in. Its value is a string.\n\
             \n\
             text       the starting text\n\
             max_len    the longest text it takes\n\
             required   it must not be left empty\n\
             validator  checks keystrokes: range, filter, picture, lookup,\n\
                        or custom (your code's)\n\
             history    an id; inputs sharing one share a history list"
        }
        "auto_complete" => {
            "An input that offers matching items as you type.\n\
             \n\
             items          the words it can offer\n\
             min_chars      how many characters before it offers\n\
             require_match  only an item is accepted"
        }
        "memo" => {
            "Several lines of text. Its value is a string.\n\
             \n\
             text       the starting text\n\
             read_only  show the text without letting it be edited\n\
             required   it must not be left empty"
        }
        "spinner" => {
            "A number with arrows to step it. Its value is a number.\n\
             \n\
             min, max  the limits\n\
             step      what an arrow adds\n\
             suffix    text after the number, such as a unit\n\
             wrap      go round from one limit to the other"
        }
        "slider" => {
            "A number chosen by sliding a marker.\n\
             \n\
             min, max  the limits\n\
             value     where it starts\n\
             step      how far an arrow moves it"
        }
        "check_box" => {
            "One option that is on or off. Its value is true or false.\n\
             \n\
             text      the caption\n\
             checked   whether it starts on\n\
             required  it must be on to accept the dialog"
        }
        "check_boxes" => {
            "Several options, any of them on. Its value is the list of\n\
             indices that are on.\n\
             \n\
             items    one caption per option\n\
             checked  the indices that start on"
        }
        "radio_buttons" => {
            "Several options, exactly one on. Its value is its index.\n\
             \n\
             items     one caption per option\n\
             selected  the index that starts on"
        }
        "combo_box" => {
            "A drop-down list of items. Its value is an index, or null.\n\
             \n\
             items     what it lists\n\
             selected  the item that starts chosen\n\
             required  an item must be chosen"
        }
        "list_box" => {
            "A list to choose from. Its value is an index, or null.\n\
             \n\
             items     what it lists\n\
             selected  the item that starts chosen\n\
             multi     choose several: the value is a list of indices"
        }
        "tree" => {
            "A tree of nodes that open and close. Its value is the key of the\n\
             selected node.\n\
             \n\
             nodes     a string, or a label with key, children and expanded\n\
             selected  the key that starts selected"
        }
        "table" => {
            "Rows and columns. Its value is the selected row, or null.\n\
             Fill it from code with set_rows.\n\
             \n\
             columns     title, width and align of each\n\
             rows        the starting rows\n\
             header      show the titles\n\
             separators  draw lines between the columns"
        }
        "dual_list" => {
            "Two lists: what is available, and what is chosen. Its value is\n\
             the list of chosen keys.\n\
             \n\
             items   strings, or a key with its label\n\
             chosen  the keys that start chosen\n\
             titles  the headings of the two lists\n\
             min, max  how many may be chosen"
        }
        "group_box" => {
            "A frame with a title around other controls. Controls inside it\n\
             are placed from its inner corner.\n\
             \n\
             Select a control and press the move key, or drag its row in the\n\
             inspector, to put it in a group box. Esc selects the box that\n\
             holds the selected control."
        }
        "tabs" => {
            "Pages of controls, one shown at a time. Tabs do not hold other\n\
             tabs.\n\
             \n\
             pages   each a title and its controls\n\
             active  the page the dialog opens on\n\
             \n\
             Click a tab, or use the page keys, to design another page."
        }
        "button" => {
            "A button that ends the dialog or runs an action.\n\
             \n\
             text     the caption\n\
             command  ok, cancel, yes, no, or an action name your code\n\
                      handles\n\
             default  the button Enter presses"
        }
        "progress_bar" => {
            "A bar that shows how far something has got.\n\
             \n\
             max           the value that fills it\n\
             value         where it starts\n\
             show_percent  print the percentage"
        }
        "error_line" => {
            "A line where the validation messages are shown when the dialog\n\
             is accepted with a mistake. It has no properties."
        }
        _ => "",
    }
}

/// The help on the dialog itself.
pub fn dialog() -> &'static str {
    "The dialog: its frame is what you are designing.\n\
     \n\
     title      the caption\n\
     width,\n\
     height     its size; drag the yellow corner to change it\n\
     resizable  whether the person can resize it when it runs\n\
     \n\
     Insert controls with the Insert menu or a right click. The arrow keys\n\
     move the dialog about the desktop while it is selected."
}

/// The help for what is selected, `None` as the dialog.
pub fn for_selection(type_name: Option<&str>) -> String {
    let (title, text) = match type_name {
        Some(t) => (t.replace('_', " "), control(t)),
        None => ("dialog".to_string(), dialog()),
    };
    format!(
        "{title}\n\n{text}\n\n\
         Enter or a double click edits the properties; the other keys are\n\
         under Help ▸ Keys."
    )
}

/// A help topic: its title in the contents and its text.
pub struct Topic {
    pub title: String,
    pub text: String,
}

fn topic(title: &str, text: &str) -> Topic {
    Topic {
        title: title.to_string(),
        text: text.to_string(),
    }
}

/// The contents: the general topics, then one per control type.
#[allow(clippy::too_many_lines)] // prose
pub fn topics(keys: &Keys) -> Vec<Topic> {
    let mut all = vec![
        topic(
            "Getting started",
            "The dialog is drawn in the middle, exactly as it will look.\n\
             \n\
             1. Insert a control: right click, the Insert menu, or the insert\n\
                key. It goes below the selected one.\n\
             2. Move it by dragging or with the arrows; resize it by its\n\
                yellow corner.\n\
             3. Press Enter (or double click) to edit its properties: its\n\
                text, its name, what it validates.\n\
             4. Test the dialog, then save it. The file is JSON that\n\
                JsonDialog::load reads.\n\
             \n\
             Give a control a name and its value comes back under that name\n\
             when the dialog is accepted.",
        ),
        topic("Keys", &crate::keys::help(keys)),
        topic(
            "Selecting and moving",
            "Click a control to select it; Tab and Shift+Tab walk them in\n\
             tab order; Esc selects the container it is in (a group box, a\n\
             tab pane, then the dialog).\n\
             \n\
             Drag a control to move it, or use the arrows; Shift+arrows and\n\
             Ctrl+arrows resize it. With the dialog selected the arrows move\n\
             the whole dialog about the desktop, and so does dragging its\n\
             frame.\n\
             \n\
             The inspector at the right lists every control as a tree. It is\n\
             the way to reach one hidden under another or on a tab page.",
        ),
        topic(
            "Containers",
            "A group box draws a frame with a title around other controls. A\n\
             tabs control holds pages; each page has its own controls. Both\n\
             place what is inside from their inner corner. Group boxes nest,\n\
             tabs do not hold tabs.\n\
             \n\
             To put a control in a container, use Edit > Move to container,\n\
             or drag its row in the inspector onto the container's row.\n\
             Selecting a container highlights its frame only, so you can see\n\
             what you are about to move.",
        ),
        topic(
            "Tab order",
            "Tab goes through the controls of a container in the order of\n\
             its list, and the data and the validation use the same order.\n\
             \n\
             View > Tab order numbers the controls. Earlier / Later move the\n\
             selected control one place. Set tab order opens its neighbours\n\
             as a list: Move up and Move down shift the one under the\n\
             cursor, OK applies the whole order as one edit that Undo takes\n\
             back.",
        ),
        topic(
            "Properties",
            "Enter, or a double click, opens the property sheet of the\n\
             selected control; with the dialog selected, the dialog's.\n\
             \n\
             General holds the control's own settings; Place its name,\n\
             position, size and hover hint; an input also has a Validator\n\
             page. Lists (items, pages, columns, rows) are one per line.\n\
             \n\
             Renaming a control carries along the labels linked to it. A\n\
             sheet that breaks a rule, such as a duplicate name, says why and\n\
             opens again with what you typed.\n\
             \n\
             F1 explains the selected control and each of its settings.",
        ),
        topic(
            "Templates",
            "File > Save as template saves the dialog under a name in the\n\
             templates folder (./templates, or $DIALOG_TEMPLATES). File >\n\
             New from template lists them: Use starts a new dialog from one,\n\
             Rename and Delete look after the folder.\n\
             \n\
             Samples come with the designer: login, contact form, find and\n\
             replace, settings, confirm, about, list picker, table browser\n\
             and progress.",
        ),
        topic(
            "Testing and using it",
            "Dialog > Test runs the dialog as an application would: type, tab,\n\
             press its buttons, and see what it answers. Dialog > Show JSON\n\
             shows what is saved.\n\
             \n\
             Dialog > Code to use it saves Rust that loads the file and binds\n\
             every named control and action; paste it into your program.\n\
             \n\
             Save writes the JSON; Open reads it back. The designer asks\n\
             before it throws away changes.",
        ),
    ];
    all.extend(KINDS.iter().map(|(name, _)| {
        topic(
            &format!("Control: {}", name.replace('_', " ")),
            control(name),
        )
    }));
    all
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_control_has_help() {
        for (name, _) in KINDS {
            assert!(!control(name).is_empty(), "{name}");
        }
    }

    #[test]
    fn the_contents_lists_the_topics_then_every_control() {
        let all = topics(&crate::keys::PC);
        assert_eq!(all.len(), 8 + KINDS.len());
        assert!(all.iter().all(|t| !t.title.is_empty() && !t.text.is_empty()));
        assert_eq!(all.last().unwrap().title, "Control: error line");
        assert!(all[1].text.contains("F9"));
    }

    #[test]
    fn the_help_names_what_is_selected() {
        assert!(for_selection(Some("check_box")).starts_with("check box"));
        assert!(for_selection(None).starts_with("dialog"));
    }
}
