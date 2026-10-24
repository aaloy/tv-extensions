//! (C) 2025 - Enzo Lombardi
//! Form Layout Demo - a data-entry dialog built with `Form`, with no coordinates.
//!
//! The form places the labels, fields, group box and buttons and sizes the
//! dialog; the handles it returns read the values back once it has closed.
//! Run with: `cargo run --example form_layout --features native`
//! Labels above their fields instead: `cargo run --example form_layout --features native -- --above`

use turbo_vision::app::Application;
use turbo_vision::core::command::CM_OK;
use turbo_vision::core::geometry::Rect;
use turbo_vision::views::GroupLike;
use turbo_vision::views::checkbox::CheckBox;
use turbo_vision::views::input_line::InputLine;
use turbo_vision::views::memo::Memo;
use turbo_vision::views::msgbox::{MsgBox, message_box};
use tv_extensions::forms::{Form, LabelAlign, LabelPosition, size};

fn main() -> turbo_vision::core::error::Result<()> {
    let mut app = Application::new()?;

    let mut form = Form::new("New Customer");
    if std::env::args().any(|a| a == "--above") {
        form.label_position(LabelPosition::Above);
    } else {
        // Labels on the left, ending against their fields.
        form.label_align(LabelAlign::Right);
    }

    // `Rect::default()`: no size, so the field stretches to the field column.
    let name = form.field("~N~ame", InputLine::new(Rect::default(), 40));
    let email = form.field("~E~mail", InputLine::new(Rect::default(), 60));

    // A titled box around the rows up to `end_group`.
    form.group("Address");
    let street = form.field("~S~treet", InputLine::new(Rect::default(), 60));
    // Two fields on one line; `size(w, h)` keeps ZIP at 8 columns and the
    // city takes the rest.
    let mut line = form.line();
    let city = line.field("~C~ity", InputLine::new(Rect::default(), 40));
    let zip = line.field("~Z~IP", InputLine::new(size(8, 1), 8));
    form.end_group();

    // An empty label: no label, but aligned with the fields above.
    let vip = form.field("", CheckBox::new(Rect::default(), "VIP customer"));

    // A heading, then a view spanning the whole form, 3 rows high.
    form.section("Notes");
    let notes = form.row(Memo::new(size(0, 3)));

    form.ok_cancel();
    let mut dialog = form.build();

    if dialog.execute(&mut app) == CM_OK {
        let text = |h| {
            dialog
                .get(h)
                .map(|f: &InputLine| f.text().to_string())
                .unwrap_or_default()
        };
        let summary = format!(
            "Name: {}\nEmail: {}\nAddress: {}, {} {}\nVIP: {}\nNotes: {} characters",
            text(name),
            text(email),
            text(street),
            text(zip),
            text(city),
            if dialog.get(vip).is_some_and(CheckBox::is_checked) {
                "yes"
            } else {
                "no"
            },
            dialog
                .get(notes)
                .map_or(0, |m| m.get_text().chars().count()),
        );
        message_box(&mut app, &summary, MsgBox::INFORMATION | MsgBox::OK_BUTTON);
    }

    Ok(())
}
