//! (C) 2025 - Enzo Lombardi
//! Form Record Demo - edit typed records with validation, saved to a "table".
//!
//! A `Form::<Customer>::for_record` binds each field to a member of the
//! `Customer` struct: text, an optional phone, an optional date, an enum
//! chosen from a list, a number, a flag and notes. The editor checks required
//! fields, conversions, field rules and a rule across fields, then hands the
//! record to the save closure. The in-memory `Customers` table stands in for
//! a database: it rejects a duplicate email the way a UNIQUE constraint
//! would, and the editor shows that error on the email field.
//!
//! Run with: `cargo run --example form_record --features native`
//! Try: OK with the form empty; "ada@example.org" as the email (taken);
//! the Team plan with 1 seat; a birthday like "1990-13-01".

use chrono::NaiveDate;
use turbo_vision::app::Application;
use turbo_vision::views::msgbox::{MsgBox, message_box};
use tv_extensions::forms::{Editor, FieldError, FieldId, Form, ValidationErrors};

/// A subscription plan: stored as an enum, chosen from a list.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
enum Plan {
    #[default]
    Free,
    Pro,
    Team,
}

/// One row of the customers table.
#[derive(Debug, Clone, Default)]
struct Customer {
    /// Set by the table on insert; the form has no field for it.
    id: Option<i64>,
    name: String,
    email: String,
    phone: Option<String>,
    birthday: Option<NaiveDate>,
    plan: Plan,
    seats: u32,
    newsletter: bool,
    notes: String,
}

/// What the table can refuse.
enum DbError {
    DuplicateEmail,
}

/// An in-memory table with a UNIQUE email, standing in for a database.
struct Customers {
    rows: Vec<Customer>,
}

impl Customers {
    fn insert(&mut self, customer: &Customer) -> Result<i64, DbError> {
        let email = customer.email.trim().to_lowercase();
        if self.rows.iter().any(|c| c.email.to_lowercase() == email) {
            return Err(DbError::DuplicateEmail);
        }
        let id = i64::try_from(self.rows.len()).unwrap_or(i64::MAX) + 1;
        self.rows.push(Customer {
            id: Some(id),
            ..customer.clone()
        });
        Ok(id)
    }
}

fn main() -> turbo_vision::core::error::Result<()> {
    let mut app = Application::new()?;
    let mut table = Customers {
        rows: vec![Customer {
            id: Some(1),
            name: "Ada Lovelace".into(),
            email: "ada@example.org".into(),
            ..Customer::default()
        }],
    };
    let (mut editor, email) = customer_editor();

    // New customers until the user cancels.
    loop {
        let blank = Customer {
            seats: 1,
            ..Customer::default()
        };
        let saved = editor.edit_with(&mut app, blank, |customer| {
            // The database's errors become field errors.
            table.insert(customer).map(drop).map_err(|e| match e {
                DbError::DuplicateEmail => {
                    FieldError::new(email, "That email is already registered").into()
                }
            })
        });
        let Some(customer) = saved else { break };
        let row = table.rows.last().and_then(|r| r.id).unwrap_or_default();
        let text = format!(
            "Saved customer #{row} ({}, {:?} plan).\nThe table now has {} customers.",
            customer.name,
            customer.plan,
            table.rows.len()
        );
        message_box(&mut app, &text, MsgBox::INFORMATION | MsgBox::OK_BUTTON);
    }
    Ok(())
}

/// The customer form: its editor, and the email field's id for the
/// database's errors.
fn customer_editor() -> (Editor<Customer>, FieldId) {
    let mut form = Form::<Customer>::for_record("New Customer");

    form.group("Contact");
    form.input("~N~ame", |c| &mut c.name).required();
    let email = form
        .input("~E~mail", |c| &mut c.email)
        .required()
        .validate(|e| {
            if e.contains('@') {
                Ok(())
            } else {
                Err("Email must contain @".into())
            }
        })
        .id();
    let mut line = form.line();
    // Option<String>: an empty field is None.
    line.input("~P~hone", |c| &mut c.phone);
    // Option<NaiveDate>: empty is None; otherwise it must be a valid date.
    line.input("~B~irthday", |c| &mut c.birthday)
        .validate(|d| match d {
            Some(d) if *d > chrono::Local::now().date_naive() => {
                Err("Birthday cannot be in the future".into())
            }
            _ => Ok(()),
        });
    form.end_group();

    form.group("Subscription");
    let mut line = form.line();
    line.choice(
        "P~l~an",
        [
            ("Free", Plan::Free),
            ("Pro", Plan::Pro),
            ("Team", Plan::Team),
        ],
        |c| &mut c.plan,
    );
    let seats = line
        .input("~S~eats", |c| &mut c.seats)
        .width(6)
        .validate(|s| {
            if (1..=500).contains(s) {
                Ok(())
            } else {
                Err("Seats must be between 1 and 500".into())
            }
        })
        .id();
    form.check("Send the ne~w~sletter", |c| &mut c.newsletter);
    form.end_group();

    form.memo("No~t~es", 3, |c| &mut c.notes);

    // A rule across fields: checked once every field is valid.
    form.validate_record(move |c, errors: &mut ValidationErrors| {
        if c.plan == Plan::Team && c.seats < 2 {
            errors.add(seats, "A Team plan needs at least 2 seats");
        }
    });

    form.ok_cancel();
    (form.build_editor(), email)
}
