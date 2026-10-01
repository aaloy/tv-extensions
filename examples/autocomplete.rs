//! Autocomplete Example
//! Demonstrates tv-extensions' `AutoComplete` and its two validation rules,
//! which combine freely:
//! - `required(true)`: the field may not be left blank.
//! - `require_match(true)`: text that is not blank must be one of the items.
//!
//! The dialog has one field per combination that adds a rule:
//! - Country, required and from the list: a country must be chosen.
//! - Fruit, from the list but optional: blank is fine, anything else must
//!   be a listed fruit.
//! - City, required but free text: anything except blank; the list only
//!   suggests.
//!
//! With neither rule, the default, any text is kept, blank included.
//!
//! OK checks the fields in order and stops at the first that breaks its
//! rule, as Turbo Vision dialogs do: the dialog stays open and that field
//! shows an error line under itself until it is edited. Fix it and press
//! OK again to check the next. Leaving a field
//! that must match fixes the spelling of a match ("méxico" becomes
//! "México").
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
use turbo_vision::core::command::{CM_CANCEL, CM_OK};
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

fn cities() -> Vec<String> {
    [
        "Amsterdam",
        "Barcelona",
        "Berlin",
        "Bilbao",
        "Lisboa",
        "London",
        "Madrid",
        "Málaga",
        "Palma",
        "Paris",
        "Porto",
        "Roma",
        "Sevilla",
        "Valencia",
        "Wien",
    ]
    .into_iter()
    .map(String::from)
    .collect()
}

fn main() -> turbo_vision::core::error::Result<()> {
    let mut app = Application::new()?;

    let (width, height) = app.terminal.size();
    let (w, h) = (62, 22);
    let x = (width - w).max(0) / 2;
    let y = (height - h).max(0) / 2;
    let mut dialog = DialogBuilder::new()
        .bounds(Rect::new(x, y, x + w, y + h))
        .title("Order")
        .build();

    dialog.add(StaticText::new(
        Rect::new(2, 1, 58, 2),
        "OK checks each field against the rule on its right.",
    ));

    // Each field leaves four rows below it: the error line and a list of up
    // to three suggestions.
    let mut row = 3;
    let mut field = |label: &str, rule: &str, field: AutoCompleteBuilder| {
        let data = Rc::new(RefCell::new(String::new()));
        dialog.add(StaticText::new(Rect::new(2, row, 12, row + 1), label));
        dialog.add(StaticText::new(Rect::new(44, row, 60, row + 1), rule));
        dialog.add(
            field
                .bounds(Rect::new(12, row, 42, row + 1))
                .data(data.clone())
                .max_drop_rows(3)
                .build(),
        );
        row += 5;
        data
    };

    // Required, and from the list: a country must be chosen.
    let country = field(
        "Country:",
        "(required, list)",
        AutoCompleteBuilder::new()
            .items(countries())
            .required(true)
            .require_match(true)
            .required_message("Choose a country")
            .match_message("Choose a country from the list"),
    );
    // From the list, but optional: blank is fine.
    let fruit = field(
        "Fruit:",
        "(optional, list)",
        AutoCompleteBuilder::new()
            .items(fruits())
            .require_match(true),
    );
    // Required, but free text: the list only suggests.
    let city = field(
        "City:",
        "(required, any)",
        AutoCompleteBuilder::new().items(cities()).required(true),
    );

    dialog.add(
        ButtonBuilder::new()
            .bounds(Rect::new(17, 18, 29, 20))
            .title("~O~K")
            .command(CM_OK)
            .default(true)
            .build(),
    );
    dialog.add(
        ButtonBuilder::new()
            .bounds(Rect::new(33, 18, 45, 20))
            .title("Cancel")
            .command(CM_CANCEL)
            .build(),
    );

    dialog.set_initial_focus();
    let result = dialog.execute(&mut app);
    drop(app); // restore the terminal before printing

    if result == CM_OK {
        println!("OK");
        println!("Country (required, list): {}", country.borrow());
        println!("Fruit (optional, list):   {}", fruit.borrow());
        println!("City (required, any):     {}", city.borrow());
    } else {
        println!("Canceled");
    }

    Ok(())
}
