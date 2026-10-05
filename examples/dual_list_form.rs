//! Dual List in a Form Example
//! Shows tv-extensions' `DualList` as one field of a larger form, the way
//! Django admin's "Change user" page puts the permissions picker among the
//! user's other fields.
//!
//! The dialog edits an existing `User` record: username, full name and email
//! (`InputLine`), a role (`RadioButton`s), status flags (`CheckBox`es) and
//! the user's permissions (`DualList`). Every field starts out with the
//! user's current values, and all of them are read back into a `User`
//! after OK: `add_fields` fills the form, `read_fields` reads it.
//!
//! Things to notice:
//! - Each field has a `Label` linked to it, the `DualList` included. Alt+P
//!   jumps to the permissions from anywhere in the form, and clicking
//!   "Permissions:" does too. Inside the `DualList`, Alt+L and Alt+H jump to
//!   its two filter fields.
//! - Tab walks the whole form in order, passing through the `DualList`'s
//!   parts on the way.
//! - Permission keys are strings (`blog.add_post`), not the labels the
//!   lists show, so the result is ready to store.
//! - OK refuses a user without permissions: the `DualList`'s header turns
//!   into an error line and the dialog stays open.
//!
//! Run with:
//!   `cargo run --example dual_list_form --features native`

// (C) 2026 - Antoni Aloy

use std::cell::RefCell;
use std::rc::Rc;

use turbo_vision::app::Application;
use turbo_vision::core::command::{CM_CANCEL, CM_OK};
use turbo_vision::core::geometry::Rect;
use turbo_vision::views::GroupLike;
use turbo_vision::views::button::ButtonBuilder;
use turbo_vision::views::checkbox::CheckBox;
use turbo_vision::views::dialog::{Dialog, DialogBuilder};
use turbo_vision::views::handle::Handle;
use turbo_vision::views::input_line::{InputLine, InputLineBuilder};
use turbo_vision::views::label::LabelBuilder;
use turbo_vision::views::radiobutton::RadioButton;
use turbo_vision::views::view::ViewId;
use tv_extensions::DualListBuilder;

/// Every permission, as Django names them: (key, label shown in the lists).
fn all_permissions() -> Vec<(String, String)> {
    let models = [
        ("auth", "group"),
        ("auth", "user"),
        ("blog", "comment"),
        ("blog", "post"),
        ("shop", "order"),
        ("shop", "product"),
    ];
    let actions = ["add", "change", "delete", "view"];
    models
        .iter()
        .flat_map(|(app, model)| {
            actions.iter().map(move |action| {
                (
                    format!("{app}.{action}_{model}"),
                    format!("{app} | {model} | Can {action} {model}"),
                )
            })
        })
        .collect()
}

const ROLES: [&str; 3] = ["Viewer", "Editor", "Administrator"];
const FLAGS: [&str; 2] = ["Active", "Staff status"];

/// The record the form edits, as it would be loaded from and saved to
/// storage.
#[derive(Debug)]
struct User {
    username: String,
    full_name: String,
    email: String,
    role: &'static str,
    status: Vec<&'static str>,
    permissions: Vec<String>,
}

/// The form's fields, kept to read the values back after the dialog closes.
struct Fields {
    username: Handle<InputLine>,
    full_name: Handle<InputLine>,
    email: Handle<InputLine>,
    roles: Vec<Handle<RadioButton>>,
    flags: Vec<Handle<CheckBox>>,
    /// The `DualList` shares its chosen keys here.
    permissions: Rc<RefCell<Vec<String>>>,
}

fn main() -> turbo_vision::core::error::Result<()> {
    let mut app = Application::new()?;

    let user = User {
        username: "jdoe".into(),
        full_name: "Jane Doe".into(),
        email: "jane@example.com".into(),
        role: "Editor",
        status: vec!["Active"],
        permissions: vec![
            "blog.add_post".into(),
            "blog.change_post".into(),
            "blog.view_comment".into(),
        ],
    };

    let (width, height) = app.terminal.size();
    let (w, h) = (78, 24);
    let x = (width - w).max(0) / 2;
    let y = (height - h).max(0) / 2;
    let mut dialog = DialogBuilder::new()
        .bounds(Rect::new(x, y, x + w, y + h))
        .title("Change user")
        .build();
    let fields = add_fields(&mut dialog, &user);
    dialog.set_initial_focus();
    let result = dialog.execute(&mut app);
    // Read every field back before the dialog goes away
    let saved = read_fields(&dialog, &fields);
    drop(app); // restore the terminal before printing

    if result == CM_OK {
        println!("Saved user");
        println!("  Username:    {}", saved.username);
        println!("  Full name:   {}", saved.full_name);
        println!("  Email:       {}", saved.email);
        println!("  Role:        {}", saved.role);
        println!("  Status:      {}", saved.status.join(", "));
        println!("  Permissions: {}", saved.permissions.join(", "));
    } else {
        println!("Canceled, nothing saved");
    }

    Ok(())
}

/// Add the form's fields to `dialog`, filled in from `user`.
fn add_fields(dialog: &mut Dialog, user: &User) -> Fields {
    // Text fields, each with a linked label
    let mut text_field = |row: i16, label: &str, value: &str| {
        let field = dialog.add_typed(
            InputLineBuilder::new()
                .bounds(Rect::new(14, row, 40, row + 1))
                .max_length(60)
                .text(value)
                .build(),
        );
        link_label(dialog, Rect::new(1, row, 13, row + 1), label, field.id());
        field
    };
    let username = text_field(1, "~U~sername:", &user.username);
    let full_name = text_field(2, "~F~ull name:", &user.full_name);
    let email = text_field(3, "~E~mail:", &user.email);

    // Role, one of three: radio buttons sharing a group id
    let roles: Vec<_> = (2..)
        .zip(ROLES)
        .map(|(row, role)| {
            let mut button = RadioButton::new(Rect::new(48, row, 74, row + 1), role, 1);
            button.set_selected(role == user.role);
            dialog.add_typed(button)
        })
        .collect();
    link_label(dialog, Rect::new(48, 1, 56, 2), "~R~ole:", roles[0].id());

    // Status flags, any of two
    let flags: Vec<_> = (5..)
        .zip(FLAGS)
        .map(|(row, flag)| {
            let mut check = CheckBox::new(Rect::new(14, row, 40, row + 1), flag);
            check.set_checked(user.status.contains(&flag));
            dialog.add_typed(check)
        })
        .collect();
    link_label(dialog, Rect::new(1, 5, 13, 6), "~S~tatus:", flags[0].id());

    // Permissions: the DualList is one more field of the form. Its own
    // titles carry hotkeys for its two filters; the dialog label above it
    // gives it a hotkey (Alt+P) that works from any other field.
    let permissions = Rc::new(RefCell::new(user.permissions.clone()));
    let picker = dialog.add_typed(
        DualListBuilder::new()
            .bounds(Rect::new(1, 9, 75, 20))
            .items(all_permissions())
            .data(permissions.clone())
            .titles("Avai~l~able permissions", "C~h~osen permissions")
            .required(true)
            .min_message("Choose at least one permission")
            .build(),
    );
    link_label(
        dialog,
        Rect::new(1, 8, 16, 9),
        "~P~ermissions:",
        picker.id(),
    );

    dialog.add(
        ButtonBuilder::new()
            .bounds(Rect::new(25, 20, 37, 22))
            .title("~O~K")
            .command(CM_OK)
            .default(true)
            .build(),
    );
    dialog.add(
        ButtonBuilder::new()
            .bounds(Rect::new(40, 20, 52, 22))
            .title("Cancel")
            .command(CM_CANCEL)
            .build(),
    );

    Fields {
        username,
        full_name,
        email,
        roles,
        flags,
        permissions,
    }
}

/// The values the form's fields hold now.
fn read_fields(dialog: &Dialog, fields: &Fields) -> User {
    let text_of = |field| {
        dialog
            .get(field)
            .map(|f: &InputLine| f.text().to_string())
            .unwrap_or_default()
    };
    User {
        username: text_of(fields.username),
        full_name: text_of(fields.full_name),
        email: text_of(fields.email),
        role: ROLES
            .iter()
            .zip(&fields.roles)
            .find(|(_, button)| dialog.get(**button).is_some_and(RadioButton::is_selected))
            .map_or("", |(role, _)| role),
        status: FLAGS
            .iter()
            .zip(&fields.flags)
            .filter(|(_, check)| dialog.get(**check).is_some_and(CheckBox::is_checked))
            .map(|(flag, _)| *flag)
            .collect(),
        permissions: fields.permissions.borrow().clone(),
    }
}

/// Add a label at `bounds` that focuses the view `link` on a click or on
/// its Alt hotkey.
fn link_label(dialog: &mut impl GroupLike, bounds: Rect, text: &str, link: ViewId) {
    let mut label = LabelBuilder::new().bounds(bounds).text(text).build();
    label.set_link(link);
    dialog.add(label);
}
