// (C) 2026 - Antoni Aloy

//! Contextual help: what the selected control is for and what its
//! properties mean.

use crate::keys::Keys;
use crate::model::KINDS;

/// What `on_change` is, for the controls that have it.
macro_rules! on_change_help {
    () => {
        "\n\
         \n\
         on_change  the name of an action. It does nothing by itself:\n\
                    your program attaches code to the name with\n\
                    dialog.on(\"name\", |ctx| { ...; Flow::Stay }), and that\n\
                    code runs each time the person changes this control.\n\
                    See Help > Contents > Reacting to changes."
    };
}

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
            concat!("A number with arrows to step it. Its value is a number.\n\
             \n\
             min, max  the limits\n\
             step      what an arrow adds\n\
             suffix    text after the number, such as a unit\n\
             wrap      go round from one limit to the other", on_change_help!())
        }
        "slider" => {
            concat!("A number chosen by sliding a marker.\n\
             \n\
             min, max  the limits\n\
             value     where it starts\n\
             step      how far an arrow moves it", on_change_help!())
        }
        "check_box" => {
            "One option that is on or off. Its value is true or false.\n\
             \n\
             text      the caption\n\
             checked   whether it starts on\n\
             required  it must be on to accept the dialog"
        }
        "check_boxes" => {
            concat!("Several options, any of them on. Its value is the list of\n\
             indices that are on.\n\
             \n\
             items    one caption per option\n\
             checked  the indices that start on", on_change_help!())
        }
        "radio_buttons" => {
            concat!("Several options, exactly one on. Its value is its index.\n\
             \n\
             items     one caption per option\n\
             selected  the index that starts on", on_change_help!())
        }
        "combo_box" => {
            concat!("A drop-down list of items. Its value is an index, or null.\n\
             \n\
             items     what it lists\n\
             selected  the item that starts chosen\n\
             required  an item must be chosen", on_change_help!())
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
            "Reacting to changes",
            "Some controls can tell your program when the person changes\n\
             them, while the dialog is still open: check boxes, radio\n\
             buttons, combo boxes, spinners and sliders. Use it to keep one\n\
             control in step with another.\n\
             \n\
             In the designer:\n\
             1. Select the control and press Enter.\n\
             2. On General, type a name in \"On change action\", for\n\
                instance volume_changed. Any name will do; it is only saved\n\
                in the JSON.\n\
             3. Press Ctrl+G (Dialog > Code to use it) to save the Rust that\n\
                loads the dialog. It has an empty handler for each name, with\n\
                a comment saying which control it is for.\n\
             \n\
             Dialog > Test runs the form but not your handlers, so changing\n\
             the control does nothing there; they run in your program.\n\
             \n\
             In your program: attach code to that name.\n\
             \n\
             dialog.on(\"volume_changed\", |ctx| {\n\
                 let level = ctx.value(\"volume\");      // the new value\n\
                 if let Some(bar) = ctx.view_mut::<ProgressBar>(\"meter\") {\n\
                     // change another control to follow it\n\
                 }\n\
                 Flow::Stay                            // keep the dialog open\n\
             });\n\
             \n\
             ctx.value(name) reads any named control; ctx.view_mut::<T>(name)\n\
             changes one. A button's action works the same way, but it runs\n\
             when the button is pressed. Dialog > Test lists the action names\n\
             nothing is bound to yet.\n\
             \n\
             A complete program: examples/dialog_on_change.rs.",
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

/// The tutorial: lessons that build a small contact form step by step,
/// each in the keys of this keyboard.
#[allow(clippy::too_many_lines)] // prose
pub fn tutorial(k: &Keys) -> Vec<Topic> {
    let lessons = [
        (
            "The screen",
            format!(
                "This tutorial builds a small contact form. Read a lesson,\n\
                 try it in the designer behind this window, then press Next.\n\
                 \n\
                 What you see:\n\
                 \n\
                 - the menu bar at the top, and the keys at the bottom;\n\
                 - in the middle the dialog you are designing. It looks just\n\
                   as it will when your program shows it;\n\
                 - at the right the inspector: a tree of the controls, over\n\
                   the JSON of the one selected.\n\
                 \n\
                 Start with File > New ({new}). Help > Keys lists every key.",
                new = "Ctrl+N"
            ),
        ),
        (
            "The dialog itself",
            "Select the dialog by clicking its frame (or press Esc until\n\
             nothing else is selected). Press Enter: the dialog's property\n\
             sheet opens.\n\
             \n\
             Set the title to \"Contact\", the width to 50 and the height to\n\
             14, and press OK. You can also drag the yellow corner at the\n\
             bottom right of the frame.\n\
             \n\
             While the dialog is selected, the arrows and dragging the frame\n\
             move it about the desktop. That changes nothing in the file; it\n\
             only makes room to work."
                .to_string(),
        ),
        (
            "A first control",
            format!(
                "Right click inside the dialog, or press {insert}. A menu\n\
                 lists the controls in sections; choose Input line.\n\
                 \n\
                 The input appears and is selected. Drag it, or use the arrows,\n\
                 to put it near the top left, leaving room on its left for a\n\
                 caption. Drag its yellow corner to make it wider.\n\
                 \n\
                 Every new control goes below the selected one, so you can\n\
                 insert several in a row and they stack up.",
                insert = k.insert
            ),
        ),
        (
            "Properties and names",
            "Press Enter on the input (or double click it). Its property\n\
             sheet has three pages: General, Place and Validator.\n\
             \n\
             On Place, give it the name \"name\". The name is how your\n\
             program finds the control: when the dialog is accepted, the\n\
             answer holds {\"name\": \"...\"}. A control with no name is\n\
             decoration and gives no value.\n\
             \n\
             On General, tick Required: the dialog will not close on OK while\n\
             this line is empty. On Validator you could limit what may be\n\
             typed; leave it alone for now."
                .to_string(),
        ),
        (
            "Captions that focus",
            "Insert a Label and put it to the left of the input. Edit it:\n\
             set its text to \"~N~ame\" - the tildes mark N as the hot key -\n\
             and, in \"Links to\", choose the input called name.\n\
             \n\
             Now Alt+N focuses the input when the dialog runs. Do the same\n\
             for an \"~E~mail\" input called email, below the first.\n\
             \n\
             Renaming an input later carries the link along, so it never\n\
             points at nothing."
                .to_string(),
        ),
        (
            "Choices",
            "Add a Radio buttons control below, named kind, with the items\n\
             Personal and Work, one per line. Its value is the index of the\n\
             chosen one, 0 for Personal.\n\
             \n\
             Add a Check box named subscribe with the text \"~S~ubscribe\".\n\
             Its value is true or false.\n\
             \n\
             Other controls choose from lists: a Combo box, a List box, a\n\
             Tree view, a Table. F1 on any control explains its settings.\n\
             \n\
             To react while the person chooses, type a name in the radio\n\
             buttons' \"On change action\", such as kind_changed. Your program\n\
             binds code to that name with dialog.on; see Help > Contents >\n\
             Reacting to changes. Without it, the value is only read when\n\
             the dialog is accepted."
                .to_string(),
        ),
        (
            "Buttons",
            "Insert two Buttons at the bottom. Make the first \"~O~K\" with the\n\
             command ok and tick Default button (Enter presses it); make\n\
             the second \"Cancel\" with the command cancel.\n\
             \n\
             OK checks the dialog's rules and closes it; Cancel closes it\n\
             with no answer. A button whose command is an action name, such\n\
             as \"clear\", runs the code you bind to that name instead."
                .to_string(),
        ),
        (
            "Tab order",
            format!(
                "Tab walks the controls in the order they were made. See it:\n\
                 press {order} and every control shows its number.\n\
                 \n\
                 To change it, select a control and press {earlier} or\n\
                 {later} to move it one place. For a bigger change press\n\
                 {set}: the controls are listed in order, and Move up and\n\
                 Move down shift the one under the cursor. OK applies it,\n\
                 and Undo ({undo}) takes it back.",
                order = k.order,
                earlier = k.earlier,
                later = k.later,
                set = k.set_order,
                undo = "Ctrl+Z"
            ),
        ),
        (
            "Containers",
            "Insert a Group box and size it to hold two controls. With the box\n\
             selected, whatever you insert goes inside it, and moves with it.\n\
             \n\
             To put a control that already exists into the box, select it\n\
             and use Edit > Move to container, or drag its row onto the\n\
             box's row in the inspector.\n\
             \n\
             Tabs work the same way, with pages: click a page's tab to design\n\
             it. Esc selects the container the selected control is in; a\n\
             selected container shows only its frame highlighted."
                .to_string(),
        ),
        (
            "Trying it out",
            format!(
                "Press {test} (Dialog > Test). The dialog runs for real: type,\n\
                 Tab, press Alt+N, tick the box, press OK. Leave Name empty to\n\
                 see the Required rule refuse it. A box tells you what the\n\
                 dialog answered.\n\
                 \n\
                 If something is wrong, close the test and change it: the\n\
                 designer is untouched by testing.\n\
                 \n\
                 {json} shows the JSON you are building.",
                test = k.test,
                json = k.json
            ),
        ),
        (
            "Saving and using it",
            format!(
                "Save with {save}, as contact.json. Your program loads it:\n\
                 \n\
                 let mut d = JsonDialog::load(\"contact.json\")?;\n\
                 if let Some(answer) = d.run(&mut app) {{\n\
                     println!(\"{{}}\", answer.data);\n\
                 }}\n\
                 \n\
                 Ctrl+G writes this, with every named control and action\n\
                 bound, to a Rust file you can paste in.\n\
                 \n\
                 Save as template keeps a dialog you will build again; New\n\
                 from template starts from one. Help > Contents explains the\n\
                 rest.",
                save = k.save
            ),
        ),
    ];
    lessons
        .into_iter()
        .enumerate()
        .map(|(n, (title, text))| Topic {
            title: format!("{}. {title}", n + 1),
            text,
        })
        .collect()
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
        assert_eq!(all.len(), 9 + KINDS.len());
        assert!(all.iter().all(|t| !t.title.is_empty() && !t.text.is_empty()));
        assert_eq!(all.last().unwrap().title, "Control: error line");
        assert!(all[1].text.contains("F9"));
    }

    #[test]
    fn the_tutorial_has_lessons_in_each_keyboards_keys() {
        let pc = tutorial(&crate::keys::PC);
        let mac = tutorial(&crate::keys::MAC);
        assert_eq!(pc.len(), 11);
        assert_eq!(pc[0].title, "1. The screen");
        let text = |l: &[Topic]| l.iter().map(|t| t.text.clone()).collect::<String>();
        assert!(text(&pc).contains("F9") && !text(&mac).contains("F9"));
        assert!(text(&mac).contains("Ctrl+E"));
    }

    #[test]
    fn the_controls_with_on_change_explain_it() {
        for name in ["check_boxes", "radio_buttons", "combo_box", "spinner", "slider"] {
            assert!(control(name).contains("on_change"), "{name}");
        }
        assert!(!control("input").contains("on_change"));
    }

    #[test]
    fn the_help_names_what_is_selected() {
        assert!(for_selection(Some("check_box")).starts_with("check box"));
        assert!(for_selection(None).starts_with("dialog"));
    }
}
