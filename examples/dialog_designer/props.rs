// (C) 2026 - Antoni Aloy

//! Property sheets: the selected control's settings, or the dialog's,
//! edited in a form. Each sheet is itself a JSON form
//! (`tv_extensions::forms::json`), filled from the spec and read back into
//! it.

use crate::model::Design;
use serde_json::{Map, Value, json};
use std::cell::RefCell;
use std::rc::Rc;
use turbo_vision::app::Application;
use turbo_vision::views::msgbox::message_box_error;
use tv_extensions::dialog_spec::{
    Align, ColumnSpec, Control, Item, Kind, LineStyle, MAX_CLUSTER_ITEMS, PageSpec, ValidatorSpec,
};
use tv_extensions::forms::json::{FormError, run};

/// Edit the selected control's properties. Returns whether the design
/// changed.
pub fn edit_control(app: &mut Application, design: &Rc<RefCell<Design>>) -> bool {
    let Some(mut control) = design.borrow().selected().cloned() else {
        return edit_dialog(app, design);
    };
    let others: Vec<String> = design
        .borrow()
        .names()
        .into_iter()
        .filter(|n| *n != control.name)
        .collect();
    loop {
        let sheet = sheet(&control, &others);
        let answers = match run(app, &sheet) {
            Ok(answers) => answers,
            Err(FormError::Cancelled) => return false,
            Err(FormError::Spec(why)) => {
                message_box_error(app, &format!("Property sheet: {why}"));
                return false;
            }
        };
        let edited = match apply(&control, &answers) {
            Ok(edited) => edited,
            Err(why) => {
                message_box_error(app, &why);
                continue;
            }
        };
        match design.borrow_mut().replace_selected(&edited) {
            Ok(()) => return true,
            Err(why) => {
                message_box_error(app, &why.0);
                // Show the sheet again with what was typed.
                control = edited;
            }
        }
    }
}

/// Edit the dialog's own settings. Returns whether the design changed.
pub fn edit_dialog(app: &mut Application, design: &Rc<RefCell<Design>>) -> bool {
    let spec = design.borrow().spec.clone();
    let sheet = json!({
        "title": "Dialog properties",
        "fields": [
            { "name": "title", "type": "text", "label": "~T~itle", "default": spec.title, "max_len": 60 },
            { "type": "row", "fields": [
                { "name": "width", "type": "integer", "label": "~W~idth", "min": 16, "max": 250, "default": spec.width },
                { "name": "height", "type": "integer", "label": "~H~eight", "min": 6, "max": 100, "default": spec.height }
            ] },
            { "name": "resizable", "type": "boolean", "label": "~R~esizable", "default": spec.resizable },
            { "name": "centered", "type": "boolean", "label": "~C~entred on the desktop",
              "default": spec.x.is_none() && spec.y.is_none() },
            { "type": "row", "fields": [
                { "name": "x", "type": "integer", "label": "~X~", "min": 0, "max": 250, "default": spec.x.unwrap_or(0) },
                { "name": "y", "type": "integer", "label": "~Y~", "min": 0, "max": 100, "default": spec.y.unwrap_or(0) }
            ] }
        ]
    });
    let Ok(a) = run(app, &sheet) else {
        return false;
    };
    let int = |key: &str| a[key].as_i64().and_then(|n| i16::try_from(n).ok());
    let at = if a["centered"] == json!(true) {
        None
    } else {
        Some((int("x").unwrap_or(0), int("y").unwrap_or(0)))
    };
    design.borrow_mut().set_dialog(
        a["title"].as_str().unwrap_or_default(),
        int("width").unwrap_or(spec.width),
        int("height").unwrap_or(spec.height),
        a["resizable"] == json!(true),
        at,
    );
    true
}

// ---- building a sheet ------------------------------------------------------

fn text(name: &str, label: &str, value: &str) -> Value {
    json!({ "name": name, "type": "text", "label": label, "default": value, "max_len": 200 })
}

fn memo(name: &str, label: &str, value: &str, rows: u8) -> Value {
    json!({ "name": name, "type": "memo", "label": label, "default": value, "rows": rows })
}

fn int(name: &str, label: &str, value: i64) -> Value {
    json!({ "name": name, "type": "integer", "label": label, "default": value })
}

fn flag(name: &str, label: &str, value: bool) -> Value {
    json!({ "name": name, "type": "boolean", "label": label, "default": value })
}

/// A choice of the indices of `items`, with `none` first when given.
fn pick(name: &str, label: &str, items: &[String], value: Option<usize>, none: bool) -> Value {
    let mut options: Vec<Value> = Vec::new();
    if none {
        options.push(json!({ "label": "(none)", "value": -1 }));
    }
    options.extend(
        items
            .iter()
            .enumerate()
            .map(|(i, item)| json!({ "label": plain(item), "value": i })),
    );
    let default = value.map_or(json!(-1), |i| json!(i));
    json!({ "name": name, "type": "choice", "label": label, "options": options, "default": default })
}

/// Tick boxes for the indices of `items`.
fn ticks(name: &str, label: &str, items: &[String], ticked: &[usize]) -> Value {
    let options: Vec<Value> = items
        .iter()
        .enumerate()
        .map(|(i, item)| json!({ "label": plain(item), "value": i }))
        .collect();
    json!({ "name": name, "type": "multichoice", "label": label, "options": options, "default": ticked })
}

fn note(text: &str) -> Value {
    json!({ "type": "note", "text": text })
}

/// Text without its `~` hot-key marks, for option captions.
fn plain(text: &str) -> String {
    text.chars().filter(|&c| c != '~').collect()
}

fn on_change(value: Option<&String>) -> Value {
    text(
        "on_change",
        "On ~c~hange action",
        value.map_or("", String::as_str),
    )
}

/// The property sheet for `c`: a page of its own settings and a page for
/// its name and place (and one for an input's validator).
#[allow(clippy::too_many_lines, reason = "one arm per control kind")]
fn sheet(c: &Control, others: &[String]) -> Value {
    let lines = |items: &[String]| items.join("\n");
    let mut general: Vec<Value> = match &c.kind {
        Kind::Label { text: t, link } => {
            let mut options = vec![json!({ "label": "(none)", "value": "" })];
            options.extend(others.iter().map(|n| json!(n)));
            vec![
                text("text", "~T~ext", t),
                json!({ "name": "link", "type": "choice", "label": "~L~inks to", "options": options,
                        "default": link.clone().unwrap_or_default() }),
                note("A ~ marks the hot key: Alt plus that letter\nfocuses the linked control."),
            ]
        }
        Kind::StaticText { text: t, centered } => vec![
            memo("text", "~T~ext", t, 4),
            flag("centered", "~C~entred", *centered),
        ],
        Kind::Button {
            text: t,
            command,
            default,
        } => {
            let standard = ["ok", "cancel", "yes", "no"];
            let (kind, action) = if standard.contains(&command.as_str()) {
                (command.as_str(), "")
            } else {
                ("action", command.as_str())
            };
            vec![
                text("text", "~T~ext", t),
                json!({ "name": "command", "type": "choice", "label": "C~o~mmand",
                        "options": ["ok", "cancel", "yes", "no", "action"], "default": kind }),
                text("action", "~A~ction name", action),
                flag("default", "~D~efault button (Enter)", *default),
                note(
                    "OK and Yes validate and close; Cancel and No close.\nAn action runs the code bound to its name.",
                ),
            ]
        }
        Kind::Input {
            text: t,
            max_len,
            required,
            history,
            ..
        } => vec![
            text("text", "Starting ~t~ext", t),
            int(
                "max_len",
                "~M~ax length",
                i64::try_from(*max_len).unwrap_or(255),
            ),
            flag("required", "~R~equired", *required),
            int(
                "history",
                "~H~istory id (0 = none)",
                i64::from(history.unwrap_or(0)),
            ),
        ],
        Kind::Memo {
            text: t,
            max_len,
            required,
            read_only,
        } => vec![
            memo("text", "Starting ~t~ext", t, 4),
            int(
                "max_len",
                "~M~ax length (0 = none)",
                max_len.map_or(0, |n| i64::try_from(n).unwrap_or(0)),
            ),
            flag("required", "~R~equired", *required),
            flag("read_only", "Read ~o~nly", *read_only),
        ],
        Kind::CheckBox {
            text: t,
            checked,
            required,
        } => vec![
            text("text", "~T~ext", t),
            flag("checked", "~C~hecked", *checked),
            flag("required", "Must be c~h~ecked", *required),
        ],
        Kind::CheckBoxes {
            items,
            checked,
            on_change: oc,
        } => {
            let mut v = vec![memo("items", "~I~tems (one per line)", &lines(items), 5)];
            if !items.is_empty() {
                v.push(ticks("checked", "C~h~ecked", items, checked));
            }
            v.push(on_change(oc.as_ref()));
            v
        }
        Kind::RadioButtons {
            items,
            selected,
            on_change: oc,
        } => {
            let mut v = vec![memo("items", "~I~tems (one per line)", &lines(items), 5)];
            if !items.is_empty() {
                v.push(pick(
                    "selected",
                    "~S~elected",
                    items,
                    Some(*selected),
                    false,
                ));
            }
            v.push(on_change(oc.as_ref()));
            v
        }
        Kind::ComboBox {
            items,
            selected,
            required,
            on_change: oc,
        } => vec![
            memo("items", "~I~tems (one per line)", &lines(items), 5),
            pick("selected", "~S~elected", items, *selected, true),
            flag("required", "~R~equired", *required),
            on_change(oc.as_ref()),
        ],
        Kind::ListBox {
            items,
            selected,
            multi,
        } => vec![
            memo("items", "~I~tems (one per line)", &lines(items), 5),
            pick("selected", "~S~elected", items, *selected, true),
            flag("multi", "~M~ultiple selection", *multi),
        ],
        Kind::Spinner {
            min,
            max,
            value,
            step,
            suffix,
            wrap,
            on_change: oc,
        } => vec![
            json!({ "type": "row", "fields": [int("min", "M~i~n", *min), int("max", "M~a~x", *max)] }),
            json!({ "type": "row", "fields": [int("value", "~V~alue", *value), int("step", "~S~tep", *step)] }),
            text("suffix", "Su~f~fix", suffix),
            flag("wrap", "~W~rap around", *wrap),
            on_change(oc.as_ref()),
        ],
        Kind::Slider {
            min,
            max,
            value,
            step,
            on_change: oc,
        } => vec![
            json!({ "type": "row", "fields": [int("min", "M~i~n", *min), int("max", "M~a~x", *max)] }),
            json!({ "type": "row", "fields": [int("value", "~V~alue", *value), int("step", "~S~tep", *step)] }),
            on_change(oc.as_ref()),
        ],
        Kind::ProgressBar {
            max,
            value,
            show_percent,
        } => vec![
            int("max", "M~a~x", i64::try_from(*max).unwrap_or(100)),
            int("value", "~V~alue", i64::try_from(*value).unwrap_or(0)),
            flag("show_percent", "Show ~p~ercentage", *show_percent),
        ],
        Kind::Table {
            columns,
            rows,
            header,
            separators,
        } => {
            let cols: Vec<String> = columns
                .iter()
                .map(|c| match c.align {
                    Align::Left => format!("{},{}", c.title, c.width),
                    Align::Right => format!("{},{},right", c.title, c.width),
                })
                .collect();
            let rows: Vec<String> = rows.iter().map(|r| r.join(",")).collect();
            vec![
                memo("columns", "~C~olumns", &lines(&cols), 4),
                note("One per line: title,width[,right]"),
                memo("rows", "~R~ows", &lines(&rows), 4),
                note("One per line, cells separated by commas"),
                flag("header", "~H~eader row", *header),
                flag("separators", "Column ~s~eparators", *separators),
            ]
        }
        Kind::GroupBox { title } => vec![text("title", "~T~itle", title)],
        Kind::Tabs { pages, active } => {
            let titles: Vec<String> = pages.iter().map(|p| p.title.clone()).collect();
            vec![
                memo("pages", "~P~ages (one title per line)", &lines(&titles), 5),
                note("Removing a line removes that page and its controls."),
                pick("active", "~S~hown first", &titles, Some(*active), false),
            ]
        }
        Kind::AutoComplete {
            items,
            text: t,
            max_len,
            min_chars,
            required,
            require_match,
        } => vec![
            memo("items", "~I~tems (one per line)", &lines(items), 5),
            text("text", "Starting ~t~ext", t),
            json!({ "type": "row", "fields": [
                int("max_len", "~M~ax length", i64::try_from(*max_len).unwrap_or(255)),
                int("min_chars", "Mi~n~ chars", i64::try_from(*min_chars).unwrap_or(1))
            ] }),
            flag("required", "~R~equired", *required),
            flag("require_match", "Must ~b~e an item", *require_match),
        ],
        Kind::DualList {
            items,
            chosen,
            titles,
            min,
            max,
        } => {
            let item_lines: Vec<String> = items
                .iter()
                .map(|i| match i {
                    Item::Plain(key) => key.clone(),
                    Item::Keyed { key, label } => format!("{key}={label}"),
                })
                .collect();
            let keys: Vec<String> = items.iter().map(|i| i.key().to_string()).collect();
            let ticked: Vec<usize> = keys
                .iter()
                .enumerate()
                .filter(|(_, k)| chosen.contains(k))
                .map(|(i, _)| i)
                .collect();
            let (available, chosen_title) = titles.clone().unwrap_or_default();
            let mut v = vec![
                memo("items", "~I~tems", &lines(&item_lines), 5),
                note("One per line: key, or key=Label"),
            ];
            if !keys.is_empty() && keys.len() <= MAX_CLUSTER_ITEMS {
                v.push(ticks("chosen", "C~h~osen", &keys, &ticked));
            }
            v.extend([
                json!({ "type": "row", "fields": [
                    text("available_title", "~A~vailable title", &available),
                    text("chosen_title", "Chosen ~t~itle", &chosen_title)
                ] }),
                json!({ "type": "row", "fields": [
                    int("min", "M~i~n chosen", i64::try_from(*min).unwrap_or(0)),
                    int("max", "M~a~x (0 = any)", max.map_or(0, |m| i64::try_from(m).unwrap_or(0)))
                ] }),
            ]);
            v
        }
        Kind::Line { style } => {
            let names: Vec<String> = LineStyle::ALL.iter().map(|s| s.name().to_string()).collect();
            let at = LineStyle::ALL.iter().position(|s| s == style);
            vec![
                pick("style", "~S~tyle", &names, at, false),
                note("One row tall: a row. One column wide: a column.\nLarger: a rectangle with corners."),
            ]
        }
        Kind::ErrorLine {} => vec![note(
            "Validation messages show here.\nWithout an error line, a message box shows them.",
        )],
    };
    if general.is_empty() {
        general.push(note("No settings."));
    }

    let place = vec![
        text("name", "~N~ame", &c.name),
        json!({ "type": "row", "fields": [
            int("x", "~X~", i64::from(c.x)), int("y", "~Y~", i64::from(c.y))
        ] }),
        json!({ "type": "row", "fields": [
            int("w", "~W~idth", i64::from(c.w)), int("h", "~H~eight", i64::from(c.h))
        ] }),
        text(
            "hint",
            "Hover hin~t~",
            c.hint.as_deref().unwrap_or_default(),
        ),
        note("The name is the control's key in the data,\nand how code reaches it."),
    ];
    let mut pages = vec![
        json!({ "label": "~G~eneral", "fields": general }),
        json!({ "label": "~P~lace", "fields": place }),
    ];
    if let Kind::Input { validator, .. } = &c.kind {
        pages.push(
            json!({ "label": "~V~alidator", "fields": validator_fields(validator.as_ref()) }),
        );
    }
    json!({
        "title": format!("{} properties", c.type_name()),
        "fields": [ { "type": "tabs", "pages": pages } ]
    })
}

fn validator_fields(v: Option<&ValidatorSpec>) -> Vec<Value> {
    let (kind, min, max, chars, mask, values, name) = match v {
        None => ("none", 0, 0, "", "", String::new(), ""),
        Some(ValidatorSpec::Range { min, max }) => ("range", *min, *max, "", "", String::new(), ""),
        Some(ValidatorSpec::Filter { chars }) => {
            ("filter", 0, 0, chars.as_str(), "", String::new(), "")
        }
        Some(ValidatorSpec::Picture { mask }) => {
            ("picture", 0, 0, "", mask.as_str(), String::new(), "")
        }
        Some(ValidatorSpec::Lookup { values }) => ("lookup", 0, 0, "", "", values.join("\n"), ""),
        Some(ValidatorSpec::Custom { name }) => {
            ("custom", 0, 0, "", "", String::new(), name.as_str())
        }
    };
    vec![
        json!({ "name": "validator", "type": "choice", "label": "~K~ind",
                "options": ["none", "range", "filter", "picture", "lookup", "custom"], "default": kind }),
        json!({ "type": "row", "fields": [int("v_min", "M~i~n", min), int("v_max", "M~a~x", max)] }),
        text("v_chars", "~A~llowed characters", chars),
        text("v_mask", "Pictu~r~e", mask),
        memo("v_values", "~L~ookup values", &values, 3),
        text("v_name", "~C~ustom name", name),
        note(
            "Only the settings of the chosen kind are used.\nPicture: # digit, @ letter, & upper, ! any.",
        ),
    ]
}

// ---- reading a sheet back ----------------------------------------------------

/// The answers of a sheet, with typed getters.
struct Answers<'a>(&'a Map<String, Value>);

impl Answers<'_> {
    fn str(&self, key: &str) -> String {
        self.0
            .get(key)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string()
    }

    fn opt_str(&self, key: &str) -> Option<String> {
        Some(self.str(key).trim().to_string()).filter(|s| !s.is_empty())
    }

    fn i64(&self, key: &str) -> Option<i64> {
        self.0.get(key).and_then(Value::as_i64)
    }

    fn i16(&self, key: &str, default: i16) -> i16 {
        self.i64(key)
            .and_then(|n| i16::try_from(n).ok())
            .unwrap_or(default)
    }

    fn usize(&self, key: &str, default: usize) -> usize {
        self.i64(key)
            .and_then(|n| usize::try_from(n).ok())
            .unwrap_or(default)
    }

    fn bool(&self, key: &str) -> bool {
        self.0.get(key) == Some(&Value::Bool(true))
    }

    /// The non-blank lines of a memo.
    fn lines(&self, key: &str) -> Vec<String> {
        self.str(key)
            .lines()
            .map(str::trim_end)
            .filter(|l| !l.trim().is_empty())
            .map(str::to_string)
            .collect()
    }

    /// A chosen index, `None` for "(none)" or one past `len`.
    fn index(&self, key: &str, len: usize) -> Option<usize> {
        self.i64(key)
            .and_then(|n| usize::try_from(n).ok())
            .filter(|&i| i < len)
    }

    fn indices(&self, key: &str, len: usize) -> Vec<usize> {
        self.0
            .get(key)
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(Value::as_u64)
                    .filter_map(|n| usize::try_from(n).ok())
                    .filter(|&i| i < len)
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// `c` with the answers of its sheet applied.
#[allow(clippy::too_many_lines, reason = "one arm per control kind")]
fn apply(c: &Control, answers: &Value) -> Result<Control, String> {
    let Value::Object(map) = answers else {
        return Err("the sheet gave no answers".into());
    };
    let a = Answers(map);
    let mut out = c.clone();
    out.name = a.str("name").trim().to_string();
    out.x = a.i16("x", c.x);
    out.y = a.i16("y", c.y);
    out.w = a.i16("w", c.w).max(1);
    out.h = a.i16("h", c.h).max(1);
    out.hint = a.opt_str("hint");
    if c.kind.has_value() && out.name.is_empty() {
        return Err(format!(
            "A {} needs a name: its key in the data.",
            c.type_name()
        ));
    }
    if out.name.contains(char::is_whitespace) {
        return Err("A name cannot contain spaces.".into());
    }

    out.kind = match &c.kind {
        Kind::Label { .. } => Kind::Label {
            text: a.str("text"),
            link: a.opt_str("link"),
        },
        Kind::StaticText { .. } => Kind::StaticText {
            text: a.str("text"),
            centered: a.bool("centered"),
        },
        Kind::Button { .. } => {
            let command = match a.str("command").as_str() {
                "action" => a
                    .opt_str("action")
                    .ok_or("An action button needs an action name.")?,
                standard => standard.to_string(),
            };
            Kind::Button {
                text: a.str("text"),
                command,
                default: a.bool("default"),
            }
        }
        Kind::Input { .. } => {
            let validator = match a.str("validator").as_str() {
                "range" => Some(ValidatorSpec::Range {
                    min: a.i64("v_min").unwrap_or(0),
                    max: a.i64("v_max").unwrap_or(0),
                }),
                "filter" => Some(ValidatorSpec::Filter {
                    chars: a
                        .opt_str("v_chars")
                        .ok_or("A filter needs its allowed characters.")?,
                }),
                "picture" => Some(ValidatorSpec::Picture {
                    mask: a
                        .opt_str("v_mask")
                        .ok_or("A picture validator needs its picture.")?,
                }),
                "lookup" => Some(ValidatorSpec::Lookup {
                    values: a.lines("v_values"),
                }),
                "custom" => Some(ValidatorSpec::Custom {
                    name: a
                        .opt_str("v_name")
                        .ok_or("A custom validator needs its name.")?,
                }),
                _ => None,
            };
            let history = a
                .i64("history")
                .and_then(|n| u16::try_from(n).ok())
                .filter(|&n| n > 0);
            Kind::Input {
                text: a.str("text"),
                max_len: a.usize("max_len", 255).max(1),
                required: a.bool("required"),
                validator,
                history,
            }
        }
        Kind::Memo { .. } => Kind::Memo {
            text: a.str("text"),
            max_len: Some(a.usize("max_len", 0)).filter(|&n| n > 0),
            required: a.bool("required"),
            read_only: a.bool("read_only"),
        },
        Kind::CheckBox { .. } => Kind::CheckBox {
            text: a.str("text"),
            checked: a.bool("checked"),
            required: a.bool("required"),
        },
        Kind::CheckBoxes { .. } => {
            let items = a.lines("items");
            Kind::CheckBoxes {
                checked: a.indices("checked", items.len()),
                items,
                on_change: a.opt_str("on_change"),
            }
        }
        Kind::RadioButtons { .. } => {
            let items = a.lines("items");
            Kind::RadioButtons {
                selected: a.index("selected", items.len()).unwrap_or(0),
                items,
                on_change: a.opt_str("on_change"),
            }
        }
        Kind::ComboBox { .. } => {
            let items = a.lines("items");
            Kind::ComboBox {
                selected: a.index("selected", items.len()),
                items,
                required: a.bool("required"),
                on_change: a.opt_str("on_change"),
            }
        }
        Kind::ListBox { .. } => {
            let items = a.lines("items");
            Kind::ListBox {
                selected: a.index("selected", items.len()),
                items,
                multi: a.bool("multi"),
            }
        }
        Kind::Spinner { .. } => Kind::Spinner {
            min: a.i64("min").unwrap_or(0),
            max: a.i64("max").unwrap_or(0),
            value: a.i64("value").unwrap_or(0),
            step: a.i64("step").unwrap_or(1).max(1),
            suffix: a.str("suffix"),
            wrap: a.bool("wrap"),
            on_change: a.opt_str("on_change"),
        },
        Kind::Slider { .. } => Kind::Slider {
            min: a.i64("min").unwrap_or(0),
            max: a.i64("max").unwrap_or(0),
            value: a.i64("value").unwrap_or(0),
            step: a.i64("step").unwrap_or(1).max(1),
            on_change: a.opt_str("on_change"),
        },
        Kind::ProgressBar { .. } => Kind::ProgressBar {
            max: a
                .i64("max")
                .and_then(|n| u64::try_from(n).ok())
                .unwrap_or(100),
            value: a
                .i64("value")
                .and_then(|n| u64::try_from(n).ok())
                .unwrap_or(0),
            show_percent: a.bool("show_percent"),
        },
        Kind::Table { .. } => {
            let columns = a
                .lines("columns")
                .iter()
                .map(|line| {
                    let parts: Vec<&str> = line.split(',').map(str::trim).collect();
                    let width = parts
                        .get(1)
                        .and_then(|w| w.parse().ok())
                        .ok_or(format!("Column \"{line}\" needs title,width."))?;
                    Ok(ColumnSpec {
                        title: parts[0].to_string(),
                        width,
                        align: if parts.get(2) == Some(&"right") {
                            Align::Right
                        } else {
                            Align::Left
                        },
                    })
                })
                .collect::<Result<Vec<_>, String>>()?;
            Kind::Table {
                columns,
                rows: a
                    .lines("rows")
                    .iter()
                    .map(|r| r.split(',').map(|cell| cell.trim().to_string()).collect())
                    .collect(),
                header: a.bool("header"),
                separators: a.bool("separators"),
            }
        }
        Kind::GroupBox { .. } => Kind::GroupBox {
            title: a.str("title"),
        },
        Kind::Tabs { pages, .. } => {
            let titles = a.lines("pages");
            let new_pages: Vec<PageSpec> = titles
                .iter()
                .enumerate()
                .map(|(i, title)| PageSpec {
                    title: title.clone(),
                    controls: pages.get(i).map(|p| p.controls.clone()).unwrap_or_default(),
                })
                .collect();
            Kind::Tabs {
                active: a.index("active", new_pages.len()).unwrap_or(0),
                pages: new_pages,
            }
        }
        Kind::AutoComplete { .. } => Kind::AutoComplete {
            items: a.lines("items"),
            text: a.str("text"),
            max_len: a.usize("max_len", 255).max(1),
            min_chars: a.usize("min_chars", 1),
            required: a.bool("required"),
            require_match: a.bool("require_match"),
        },
        Kind::DualList { chosen, .. } => {
            let items: Vec<Item> = a
                .lines("items")
                .iter()
                .map(|line| match line.split_once('=') {
                    Some((key, label)) => Item::Keyed {
                        key: key.trim().to_string(),
                        label: label.trim().to_string(),
                    },
                    None => Item::Plain(line.trim().to_string()),
                })
                .collect();
            let keys: Vec<String> = items.iter().map(|i| i.key().to_string()).collect();
            let chosen = if map.contains_key("chosen") {
                a.indices("chosen", keys.len())
                    .into_iter()
                    .map(|i| keys[i].clone())
                    .collect()
            } else {
                chosen
                    .iter()
                    .filter(|k| keys.contains(k))
                    .cloned()
                    .collect()
            };
            let titles = match (a.opt_str("available_title"), a.opt_str("chosen_title")) {
                (None, None) => None,
                (av, ch) => Some((
                    av.unwrap_or_else(|| "Available".into()),
                    ch.unwrap_or_else(|| "Chosen".into()),
                )),
            };
            Kind::DualList {
                items,
                chosen,
                titles,
                min: a.usize("min", 0),
                max: Some(a.usize("max", 0)).filter(|&m| m > 0),
            }
        }
        Kind::Line { style } => Kind::Line {
            style: a
                .index("style", LineStyle::ALL.len())
                .map_or(*style, |i| LineStyle::ALL[i]),
        },
        Kind::ErrorLine {} => Kind::ErrorLine {},
    };
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::KINDS;

    /// Every kind's sheet is a valid form, and reading its own defaults
    /// back changes nothing.
    #[test]
    fn every_sheet_builds_and_round_trips() {
        let mut design = Design::default();
        for (type_name, _) in KINDS {
            design.select(crate::model::Sel::Dialog);
            design.insert(type_name);
            let control = design.selected().unwrap().clone();
            let sheet = sheet(&control, &design.names());
            let form = tv_extensions::forms::json::JsonForm::from_spec(&sheet);
            assert!(form.is_ok(), "{type_name}: {:?}", form.err());
            let answers = defaults(&sheet);
            let back = apply(&control, &answers).unwrap();
            assert_eq!(back, control, "{type_name}");
        }
    }

    /// The answers a sheet gives when nothing is touched: each default.
    fn defaults(sheet: &Value) -> Value {
        fn walk(fields: &[Value], out: &mut Map<String, Value>) {
            for f in fields {
                if let Some(name) = f.get("name").and_then(Value::as_str) {
                    let value = f.get("default").cloned().unwrap_or(Value::Null);
                    out.insert(name.to_string(), value);
                }
                if let Some(inner) = f.get("fields").and_then(Value::as_array) {
                    walk(inner, out);
                }
                if let Some(pages) = f.get("pages").and_then(Value::as_array) {
                    for p in pages {
                        walk(p["fields"].as_array().unwrap(), out);
                    }
                }
            }
        }
        let mut out = Map::new();
        walk(sheet["fields"].as_array().unwrap(), &mut out);
        Value::Object(out)
    }
}
