//! (C) 2025 - Enzo Lombardi
//! Form Labels Demo - the same form with its labels in each position.
//!
//! A launcher (itself a `Form`) opens one contact form three ways: labels on
//! the left, labels on the left but right-aligned, and labels above their
//! fields. The form has a line of two fields and a group, to show how each
//! style treats them. Nothing else changes between the three: only the
//! `label_position` / `label_align` calls.
//! Run with: `cargo run --example form_labels --features native`

use turbo_vision::app::Application;
use turbo_vision::core::command::{CM_CANCEL, CM_OK, CommandId};
use turbo_vision::core::geometry::Rect;
use turbo_vision::views::dialog::Dialog;
use turbo_vision::views::input_line::InputLine;
use turbo_vision::views::msgbox::{MsgBox, message_box};
use turbo_vision::views::static_text::StaticText;
use turbo_vision::views::{GroupLike, Handle};
use tv_extensions::forms::{Form, LabelAlign, LabelPosition, size};

const CM_LEFT: CommandId = 1001;
const CM_RIGHT: CommandId = 1002;
const CM_ABOVE: CommandId = 1003;

/// The three label styles a form can use.
#[derive(Clone, Copy)]
enum Style {
    /// The default: a label column, labels against its left edge.
    Left,
    /// A label column, labels against the fields.
    RightAligned,
    /// Each label on the row above its field.
    Above,
}

fn main() -> turbo_vision::core::error::Result<()> {
    let mut app = Application::new()?;

    loop {
        let style = match launcher().execute(&mut app) {
            CM_LEFT => Style::Left,
            CM_RIGHT => Style::RightAligned,
            CM_ABOVE => Style::Above,
            _ => break, // Close, Esc, or the close box
        };
        let mut contact = contact_form(style);
        if contact.dialog.execute(&mut app) == CM_OK {
            let text = |h| {
                contact
                    .dialog
                    .get(h)
                    .map(|f: &InputLine| f.text().to_string())
                    .unwrap_or_default()
            };
            let summary = format!(
                "Saved: {} {} <{}>",
                text(contact.first),
                text(contact.last),
                text(contact.email)
            );
            message_box(&mut app, &summary, MsgBox::INFORMATION | MsgBox::OK_BUTTON);
        }
    }
    Ok(())
}

/// The launcher: a short note and one button per style. A `Form` with no
/// fields is a quick way to make a button dialog.
fn launcher() -> Dialog {
    let mut form = Form::new("Label Positions");
    form.row(StaticText::new(
        size(0, 2),
        "Open the same form with its labels\nin each position:",
    ));
    form.default_button("~L~eft", CM_LEFT);
    form.button("~R~ight", CM_RIGHT);
    form.button("~A~bove", CM_ABOVE);
    form.button("Close", CM_CANCEL);
    form.build()
}

/// The contact form, and handles to the fields read back after it closes.
struct Contact {
    dialog: Dialog,
    first: Handle<InputLine>,
    last: Handle<InputLine>,
    email: Handle<InputLine>,
}

/// One contact form, laid out in `style`.
fn contact_form(style: Style) -> Contact {
    let title = match style {
        Style::Left => "Contact - labels left",
        Style::RightAligned => "Contact - labels right-aligned",
        Style::Above => "Contact - labels above",
    };
    let mut form = Form::new(title);

    // The only lines that differ between the three styles.
    match style {
        Style::Left => {} // the default
        Style::RightAligned => {
            form.label_align(LabelAlign::Right);
        }
        Style::Above => {
            form.label_position(LabelPosition::Above);
        }
    }

    let input = || InputLine::new(Rect::default(), 40);

    // Two fields on one line.
    let mut line = form.line();
    let first = line.field("~F~irst name", input());
    let last = line.field("~L~ast name", input());

    let email = form.field("~E~mail", input());
    form.field("~P~hone", InputLine::new(size(15, 1), 15));

    // A group lines up its own labels.
    form.group("Address");
    form.field("~S~treet", input());
    let mut line = form.line();
    line.field("~C~ity", input());
    line.field("~Z~IP", InputLine::new(size(8, 1), 8));
    form.end_group();

    form.ok_cancel();
    Contact {
        dialog: form.build(),
        first,
        last,
        email,
    }
}
