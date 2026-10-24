//! Builds a dialog from labelled fields, with no coordinates: labels in one
//! column, fields in the next, buttons at the bottom, the dialog sized and
//! centred. The full guide is docs/forms.md.
//!
//! Parameters:
//! - `Form::new(title)`, then `build()`: the dialog, to run with
//!   `execute(app)`.
//! - `field(label, view)`: one labelled row; the label's ~letter~ focuses
//!   the view. A view built with `Rect::default()` stretches to the field
//!   column; `size(w, h)` keeps a size.
//! - `line()`: fields side by side on one row. `row(view)`: a view across
//!   the whole width, with no label.
//! - `group(title)` ... `end_group()`: a box around the rows between.
//!   `section(title)`: a heading. `gap(n)`: blank rows.
//! - `ok_cancel()`, `button(title, cmd)`, `default_button(title, cmd)`: the
//!   bottom row's buttons.
//! - `spacing(n)`, `field_width(w)`, `label_position`, `label_align`,
//!   `button_align`, `resizable(on)`: the layout.
//! - `Form::<T>::for_record(title)`: binds the fields to a struct of yours
//!   (`input`, `check`, `memo`, `choice`) and validates them.
//!
//! See also: Form from JSON, DualList

use crate::panel::Panel;
use turbo_vision::app::Application;
use turbo_vision::core::command::{CM_OK, CM_USER, CommandId};
use turbo_vision::core::geometry::Rect;
use turbo_vision::views::GroupLike;
use turbo_vision::views::button::Button;
use turbo_vision::views::checkbox::CheckBox;
use turbo_vision::views::input_line::InputLine;
use turbo_vision::views::msgbox::message_box_ok;
use tv_extensions::forms::{Form, size};

const OPEN: CommandId = CM_USER + 100;

pub fn build(panel: &mut Panel) {
    panel.add(Button::new(
        Rect::new(0, 0, 18, 2),
        "~O~pen a form",
        OPEN,
        true,
    ));
}

pub fn handle(app: &mut Application, command: CommandId) -> bool {
    if command != OPEN {
        return false;
    }
    let mut form = Form::new("New Customer");
    let name = form.field("~N~ame", InputLine::new(Rect::default(), 40));
    form.group("Address");
    form.field("~S~treet", InputLine::new(Rect::default(), 40));
    let mut line = form.line();
    line.field("~C~ity", InputLine::new(Rect::default(), 30));
    line.field("~Z~IP", InputLine::new(size(8, 1), 8));
    form.end_group();
    form.field("", CheckBox::new(Rect::default(), "~V~IP customer"));
    form.ok_cancel();

    let mut dialog = form.build();
    if dialog.execute(app) == CM_OK {
        let name = dialog
            .get(name)
            .map(|f| f.text().to_string())
            .unwrap_or_default();
        message_box_ok(app, &format!("Saved: {name}"));
    }
    true
}
