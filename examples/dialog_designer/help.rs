// (C) 2026 - Antoni Aloy

//! Contextual help: what the selected control is for and what its
//! properties mean.

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::KINDS;

    #[test]
    fn every_control_has_help() {
        for (name, _) in KINDS {
            assert!(!control(name).is_empty(), "{name}");
        }
    }

    #[test]
    fn the_help_names_what_is_selected() {
        assert!(for_selection(Some("check_box")).starts_with("check box"));
        assert!(for_selection(None).starts_with("dialog"));
    }
}
