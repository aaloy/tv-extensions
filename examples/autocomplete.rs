//! Autocomplete Example
//! Demonstrates tv-extensions' `AutoComplete` in its two modes, side by
//! side in one dialog:
//! - Country requires a match (`require_match(true)`): the value must be
//!   one of the countries. OK will not close the dialog while the field is
//!   empty or holds anything else; an error line appears under the field
//!   until it is fixed. Leaving the field fixes the spelling of a match
//!   ("méxico" becomes "México").
//! - Fruit takes free text (the default): the list only suggests, and any
//!   text is kept.
//!
//! Run with:
//!   `cargo run --example autocomplete --features native`
//!
//! Type to filter (accented Latin-1 letters work too); Up/Down move the
//! highlight, Enter or a click accepts a suggestion, Esc closes the list.
//! Tab moves between the fields and buttons. The values are printed when
//! the dialog closes.

// (C) 2026 - Antoni Aloy

use std::cell::RefCell;
use std::rc::Rc;

use turbo_vision::app::Application;
use turbo_vision::core::command::CM_OK;
use turbo_vision::core::geometry::Rect;
use turbo_vision::views::GroupLike;
use turbo_vision::views::button::ButtonBuilder;
use turbo_vision::views::dialog::DialogBuilder;
use turbo_vision::views::static_text::StaticText;
use tv_extensions::AutoCompleteBuilder;

fn countries() -> Vec<String> {
    [
        "Argentina",
        "Australia",
        "Austria",
        "Belgium",
        "Brazil",
        "Canada",
        "Chile",
        "China",
        "Denmark",
        "España",
        "Finland",
        "France",
        "Germany",
        "Greece",
        "Iceland",
        "India",
        "Indonesia",
        "Ireland",
        "Italy",
        "Japan",
        "Kenya",
        "México",
        "Netherlands",
        "New Zealand",
        "Norway",
        "Perú",
        "Poland",
        "Portugal",
        "Sweden",
        "Switzerland",
        "Thailand",
        "Türkiye",
        "United Kingdom",
        "United States",
        "Uruguay",
        "Vietnam",
    ]
    .into_iter()
    .map(String::from)
    .collect()
}

fn fruits() -> Vec<String> {
    [
        "Apple",
        "Apricot",
        "Banana",
        "Blackberry",
        "Blueberry",
        "Cherry",
        "Clementine",
        "Cranberry",
        "Date",
        "Fig",
        "Grape",
        "Grapefruit",
        "Guava",
        "Kiwi",
        "Lemon",
        "Lime",
        "Mango",
        "Melon",
        "Nectarine",
        "Orange",
        "Papaya",
        "Peach",
        "Pear",
        "Pineapple",
        "Plum",
        "Pomegranate",
        "Raspberry",
        "Strawberry",
        "Tangerine",
        "Watermelon",
    ]
    .into_iter()
    .map(String::from)
    .collect()
}

fn main() -> turbo_vision::core::error::Result<()> {
    let mut app = Application::new()?;

    let (width, height) = app.terminal.size();
    let (w, h) = (62, 20);
    let x = (width - w).max(0) / 2;
    let y = (height - h).max(0) / 2;
    let mut dialog = DialogBuilder::new()
        .bounds(Rect::new(x, y, x + w, y + h))
        .title("Order")
        .build();

    dialog.add(StaticText::new(
        Rect::new(2, 1, 58, 2),
        "Country must come from the list; fruit can be anything.",
    ));

    // Strict: only a listed country. Each field's list opens below it, so
    // the fields leave room for it (Country shows up to six rows).
    dialog.add(StaticText::new(Rect::new(2, 3, 12, 4), "Country:"));
    let country = Rc::new(RefCell::new(String::new()));
    dialog.add(StaticText::new(Rect::new(44, 3, 58, 4), "(from list)"));
    dialog.add(
        AutoCompleteBuilder::new()
            .bounds(Rect::new(12, 3, 42, 4))
            .items(countries())
            .data(country.clone())
            .require_match(true)
            .error_message("Choose a country from the list")
            .build(),
    );

    // Free text: the list only suggests.
    dialog.add(StaticText::new(Rect::new(2, 11, 12, 12), "Fruit:"));
    let fruit = Rc::new(RefCell::new(String::new()));
    dialog.add(StaticText::new(Rect::new(44, 11, 58, 12), "(free text)"));
    dialog.add(
        AutoCompleteBuilder::new()
            .bounds(Rect::new(12, 11, 42, 12))
            .items(fruits())
            .data(fruit.clone())
            .max_drop_rows(4)
            .build(),
    );

    dialog.add(
        ButtonBuilder::new()
            .bounds(Rect::new(17, 16, 29, 18))
            .title("~O~K")
            .command(CM_OK)
            .default(true)
            .build(),
    );
    dialog.add(
        ButtonBuilder::new()
            .bounds(Rect::new(33, 16, 45, 18))
            .title("Cancel")
            .command(turbo_vision::core::command::CM_CANCEL)
            .build(),
    );

    dialog.set_initial_focus();
    let result = dialog.execute(&mut app);
    drop(app); // restore the terminal before printing

    if result == CM_OK {
        println!("OK");
        println!("Country (from list): {}", country.borrow());
        println!("Fruit (free text):   {}", fruit.borrow());
    } else {
        println!("Canceled");
    }

    Ok(())
}
