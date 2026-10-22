// (C) 2026 - Antoni Aloy

//! The spec: what a designed dialog holds, as saved in JSON.

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{Map, Value};
use std::collections::HashSet;
use std::fmt;
use std::path::Path;

/// The spec format this version of the crate writes, in `"version"`.
pub const VERSION: u32 = 1;

/// The most items a check-box or radio-button cluster holds.
pub const MAX_CLUSTER_ITEMS: usize = turbo_vision::views::cluster_group::MAX_CLUSTER_ITEMS;

/// Why a spec could not be read, written or used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpecError(pub String);

impl fmt::Display for SpecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid dialog spec: {}", self.0)
    }
}

impl std::error::Error for SpecError {}

/// A designed dialog: its frame and the controls inside it.
///
/// `width` and `height` are the dialog's outer size, frame included. Each
/// control's place is measured from the corner *inside* the frame, as in
/// any Turbo Vision dialog.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DialogSpec {
    /// The spec format, [`VERSION`].
    #[serde(default = "version")]
    pub version: u32,
    /// The title in the frame.
    #[serde(default)]
    pub title: String,
    /// Outer width, frame included.
    pub width: i16,
    /// Outer height, frame included.
    pub height: i16,
    /// The column of the dialog's left edge on the desktop; centred when
    /// absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x: Option<i16>,
    /// The row of the dialog's top edge on the desktop; centred when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y: Option<i16>,
    /// Whether the person can resize the dialog.
    #[serde(default, skip_serializing_if = "is_false")]
    pub resizable: bool,
    /// The controls, in tab order.
    #[serde(default)]
    pub controls: Vec<Control>,
}

fn version() -> u32 {
    VERSION
}

#[allow(
    clippy::trivially_copy_pass_by_ref,
    reason = "serde's skip_serializing_if signature"
)]
fn is_false(b: &bool) -> bool {
    !*b
}

#[allow(
    clippy::trivially_copy_pass_by_ref,
    reason = "serde's skip_serializing_if signature"
)]
fn is_zero(n: &i64) -> bool {
    *n == 0
}

#[allow(
    clippy::trivially_copy_pass_by_ref,
    reason = "serde's skip_serializing_if signature"
)]
fn is_one(n: &i64) -> bool {
    *n == 1
}

#[allow(
    clippy::trivially_copy_pass_by_ref,
    reason = "serde's skip_serializing_if signature"
)]
fn is_true(b: &bool) -> bool {
    *b
}

fn yes() -> bool {
    true
}

fn one() -> i64 {
    1
}

fn hundred() -> u64 {
    100
}

fn input_max_len() -> usize {
    255
}

#[allow(
    clippy::trivially_copy_pass_by_ref,
    reason = "serde's skip_serializing_if signature"
)]
fn is_input_max_len(n: &usize) -> bool {
    *n == input_max_len()
}

fn ok() -> String {
    "ok".to_string()
}

fn min_chars() -> usize {
    1
}

#[allow(
    clippy::trivially_copy_pass_by_ref,
    reason = "serde's skip_serializing_if signature"
)]
fn is_min_chars(n: &usize) -> bool {
    *n == min_chars()
}

impl DialogSpec {
    /// An empty dialog of this outer size, centred on the desktop.
    #[must_use]
    pub fn new(title: &str, width: i16, height: i16) -> Self {
        Self {
            version: VERSION,
            title: title.to_string(),
            width,
            height,
            x: None,
            y: None,
            resizable: false,
            controls: Vec::new(),
        }
    }

    /// Read a spec from JSON and [`check`](Self::check) it.
    ///
    /// # Errors
    ///
    /// The first thing wrong: bad JSON, an unknown control type or key (a
    /// typo is an error, not silently ignored), a missing place, or a rule
    /// [`check`](Self::check) enforces.
    pub fn from_json(text: &str) -> Result<Self, SpecError> {
        let spec: Self = serde_json::from_str(text).map_err(|e| SpecError(e.to_string()))?;
        spec.check()?;
        Ok(spec)
    }

    /// Read a spec from a JSON value and [`check`](Self::check) it.
    ///
    /// # Errors
    ///
    /// As [`from_json`](Self::from_json).
    pub fn from_value(value: &Value) -> Result<Self, SpecError> {
        let spec = Self::deserialize(value).map_err(|e| SpecError(e.to_string()))?;
        spec.check()?;
        Ok(spec)
    }

    /// The spec as pretty-printed JSON, with values left at their defaults
    /// left out.
    ///
    /// # Panics
    ///
    /// Never: every spec has a JSON form.
    #[must_use]
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("a spec always serializes")
    }

    /// Read and check the spec saved in the file at `path`.
    ///
    /// # Errors
    ///
    /// The file cannot be read, or holds no valid spec.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, SpecError> {
        let path = path.as_ref();
        let text = std::fs::read_to_string(path)
            .map_err(|e| SpecError(format!("{}: {e}", path.display())))?;
        Self::from_json(&text)
    }

    /// Save the spec as JSON in the file at `path`.
    ///
    /// # Errors
    ///
    /// The file cannot be written.
    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), SpecError> {
        let path = path.as_ref();
        let mut text = self.to_json();
        text.push('\n');
        std::fs::write(path, text).map_err(|e| SpecError(format!("{}: {e}", path.display())))
    }

    /// The control named `name`, wherever it is (on a tab page too).
    #[must_use]
    pub fn find(&self, name: &str) -> Option<&Control> {
        let mut found = None;
        self.visit(&mut |c| {
            if found.is_none() && !name.is_empty() && c.name == name {
                found = Some(c);
            }
        });
        found
    }

    /// Call `f` with every control, tab pages' controls after their tabs.
    pub fn visit<'a>(&'a self, f: &mut impl FnMut(&'a Control)) {
        visit(&self.controls, f);
    }

    /// The size of the area inside the frame, where controls go.
    #[must_use]
    pub fn inner_size(&self) -> (i16, i16) {
        ((self.width - 2).max(0), (self.height - 2).max(0))
    }

    /// Check the rules JSON alone cannot express: names are unique (across
    /// tab pages too), labels link to a control that exists, tabs do not
    /// nest, a selection is one of the items, a range is not upside down.
    ///
    /// # Errors
    ///
    /// The first rule broken, naming the control.
    pub fn check(&self) -> Result<(), SpecError> {
        if self.version > VERSION {
            return Err(SpecError(format!(
                "version {} is newer than this program reads ({VERSION})",
                self.version
            )));
        }
        let mut names = HashSet::new();
        let mut first_error = None;
        self.visit(&mut |c| {
            if first_error.is_some() {
                return;
            }
            if !c.name.is_empty() && !names.insert(c.name.as_str()) {
                first_error = Some(format!("two controls are named \"{}\"", c.name));
            } else if let Err(why) = c.check() {
                first_error = Some(format!("{}: {why}", c.describe()));
            }
        });
        if let Some(why) = first_error {
            return Err(SpecError(why));
        }
        for page in self.controls.iter().filter_map(|c| match &c.kind {
            Kind::Tabs { pages, .. } => Some((c, pages)),
            _ => None,
        }) {
            let (tabs, pages) = page;
            if pages
                .iter()
                .flat_map(|p| &p.controls)
                .any(|c| matches!(c.kind, Kind::Tabs { .. }))
            {
                return Err(SpecError(format!(
                    "{}: tabs cannot hold other tabs",
                    tabs.describe()
                )));
            }
        }
        let mut bad_link = None;
        self.visit(&mut |c| {
            if let Kind::Label {
                link: Some(target), ..
            } = &c.kind
                && bad_link.is_none()
                && !names.contains(target.as_str())
            {
                bad_link = Some(format!(
                    "{}: links to \"{target}\", which is not a control",
                    c.describe()
                ));
            }
        });
        bad_link.map_or(Ok(()), |why| Err(SpecError(why)))
    }
}

fn visit<'a>(controls: &'a [Control], f: &mut impl FnMut(&'a Control)) {
    for control in controls {
        f(control);
        if let Kind::Tabs { pages, .. } = &control.kind {
            for page in pages {
                visit(&page.controls, f);
            }
        }
    }
}

/// One control: its name, its place, and what it is.
///
/// Saved as one flat JSON object: `type`, `name`, `x`, `y`, `w`, `h`, an
/// optional `hint`, then the keys of its [`Kind`].
#[derive(Debug, Clone, PartialEq)]
pub struct Control {
    /// The key of its value in the dialog's data, and how code finds it.
    /// Empty for a control nobody needs to reach, such as most labels.
    pub name: String,
    /// Column, from the inner left edge of the dialog (or tab page).
    pub x: i16,
    /// Row, from the inner top edge of the dialog (or tab page).
    pub y: i16,
    /// Width.
    pub w: i16,
    /// Height.
    pub h: i16,
    /// A hover hint shown for the control (a tooltip).
    pub hint: Option<String>,
    /// What the control is, and its settings.
    pub kind: Kind,
}

impl Control {
    /// A control named `name` at `(x, y, w, h)`.
    #[must_use]
    pub fn new(name: &str, (x, y, w, h): (i16, i16, i16, i16), kind: Kind) -> Self {
        Self {
            name: name.to_string(),
            x,
            y,
            w,
            h,
            hint: None,
            kind,
        }
    }

    /// The control's type as written in JSON, such as `"input"`.
    #[must_use]
    pub fn type_name(&self) -> &'static str {
        self.kind.type_name()
    }

    /// How messages name the control: `input "email"`, or `button "~O~K"`
    /// for one without a name.
    #[must_use]
    pub fn describe(&self) -> String {
        if self.name.is_empty() {
            match self.kind.caption() {
                Some(text) => format!("{} \"{text}\"", self.type_name()),
                None => self.type_name().to_string(),
            }
        } else {
            format!("{} \"{}\"", self.type_name(), self.name)
        }
    }

    fn check(&self) -> Result<(), String> {
        if self.w < 0 || self.h < 0 {
            return Err("has a negative size".into());
        }
        let cluster = |items: &[String]| {
            if items.len() > MAX_CLUSTER_ITEMS {
                Err(format!("has more than {MAX_CLUSTER_ITEMS} items"))
            } else {
                Ok(())
            }
        };
        let within = |index: usize, len: usize| {
            if index < len {
                Ok(())
            } else {
                Err(format!("selects item {index} of {len}"))
            }
        };
        let range = |min: i64, max: i64| {
            if min <= max {
                Ok(())
            } else {
                Err(format!("has min {min} above max {max}"))
            }
        };
        match &self.kind {
            Kind::CheckBoxes { items, checked, .. } => {
                cluster(items)?;
                checked.iter().try_for_each(|&i| within(i, items.len()))
            }
            Kind::RadioButtons {
                items, selected, ..
            } => {
                cluster(items)?;
                if items.is_empty() {
                    Ok(())
                } else {
                    within(*selected, items.len())
                }
            }
            Kind::ComboBox {
                items,
                selected: Some(i),
                ..
            }
            | Kind::ListBox {
                items,
                selected: Some(i),
                ..
            } => within(*i, items.len()),
            Kind::Spinner { min, max, .. }
            | Kind::Slider { min, max, .. }
            | Kind::Input {
                validator: Some(ValidatorSpec::Range { min, max }),
                ..
            } => range(*min, *max),
            Kind::Tree { nodes, selected } => {
                let mut keys = HashSet::new();
                let mut dup = None;
                TreeNode::each(nodes, &mut |n| {
                    if !keys.insert(n.key().to_string()) && dup.is_none() {
                        dup = Some(n.key().to_string());
                    }
                });
                if let Some(key) = dup {
                    return Err(format!("has the node key \"{key}\" twice"));
                }
                match selected {
                    Some(key) if !keys.contains(key.as_str()) => {
                        Err(format!("selects \"{key}\", which is not a node"))
                    }
                    _ => Ok(()),
                }
            }
            Kind::Button { command, .. } if command.trim().is_empty() => {
                Err("has no command".into())
            }
            Kind::Tabs { pages, active, .. } if !pages.is_empty() => within(*active, pages.len()),
            _ => Ok(()),
        }
    }
}

/// What a control is, with the settings of that kind. The JSON `type` is
/// the variant's name in `snake_case`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Kind {
    /// A field's caption. Alt plus its `~` letter, or a click, focuses the
    /// `link`ed control, and the label turns red while that control is
    /// invalid.
    Label {
        /// The caption; a `~` marks the hot key.
        text: String,
        /// The name of the control it captions.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        link: Option<String>,
    },
    /// Fixed text, wrapped to its width.
    StaticText {
        /// The text; `\n` starts a new line.
        text: String,
        /// Centre each line.
        #[serde(default, skip_serializing_if = "is_false")]
        centered: bool,
    },
    /// A push button.
    Button {
        /// The caption; a `~` marks the hot key.
        text: String,
        /// `"ok"`, `"cancel"`, `"yes"` or `"no"`, or the name of an action
        /// your code binds (see [`JsonDialog::on`](super::JsonDialog::on)).
        #[serde(default = "ok")]
        command: String,
        /// Enter presses it.
        #[serde(default, skip_serializing_if = "is_false")]
        default: bool,
    },
    /// One line of text.
    Input {
        /// The text shown at first.
        #[serde(default, skip_serializing_if = "String::is_empty")]
        text: String,
        /// The most characters it takes.
        #[serde(default = "input_max_len", skip_serializing_if = "is_input_max_len")]
        max_len: usize,
        /// It may not be left blank.
        #[serde(default, skip_serializing_if = "is_false")]
        required: bool,
        /// What it accepts as it is typed.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        validator: Option<ValidatorSpec>,
        /// A history list id: adds the drop-down of earlier entries to its
        /// right.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        history: Option<u16>,
    },
    /// Several lines of text.
    Memo {
        /// The text shown at first.
        #[serde(default, skip_serializing_if = "String::is_empty")]
        text: String,
        /// The most characters it takes.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        max_len: Option<usize>,
        /// It may not be left blank.
        #[serde(default, skip_serializing_if = "is_false")]
        required: bool,
        /// Shown, not edited.
        #[serde(default, skip_serializing_if = "is_false")]
        read_only: bool,
    },
    /// One on/off option.
    CheckBox {
        /// The caption; a `~` marks the hot key.
        text: String,
        /// Checked at first.
        #[serde(default, skip_serializing_if = "is_false")]
        checked: bool,
        /// It must be checked (an "I agree" box).
        #[serde(default, skip_serializing_if = "is_false")]
        required: bool,
    },
    /// Several on/off options in one control, one per row.
    CheckBoxes {
        /// The captions.
        items: Vec<String>,
        /// The indices checked at first.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        checked: Vec<usize>,
        /// An action run whenever the person changes a box.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        on_change: Option<String>,
    },
    /// One choice of several, one per row.
    RadioButtons {
        /// The captions.
        items: Vec<String>,
        /// The index chosen at first.
        #[serde(default)]
        selected: usize,
        /// An action run whenever the person changes the choice.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        on_change: Option<String>,
    },
    /// One choice from a drop-down list.
    ComboBox {
        /// The choices.
        items: Vec<String>,
        /// The index chosen at first; none when absent.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        selected: Option<usize>,
        /// Something must be chosen.
        #[serde(default, skip_serializing_if = "is_false")]
        required: bool,
        /// An action run whenever the person changes the choice.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        on_change: Option<String>,
    },
    /// A scrolling list of strings.
    ListBox {
        /// The items.
        items: Vec<String>,
        /// The index selected at first.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        selected: Option<usize>,
        /// Several items can be marked (Space or Insert).
        #[serde(default, skip_serializing_if = "is_false")]
        multi: bool,
    },
    /// A whole number with up and down steppers.
    Spinner {
        /// The lowest value.
        #[serde(default, skip_serializing_if = "is_zero")]
        min: i64,
        /// The highest value.
        max: i64,
        /// The value at first.
        #[serde(default, skip_serializing_if = "is_zero")]
        value: i64,
        /// How much one step changes it.
        #[serde(default = "one", skip_serializing_if = "is_one")]
        step: i64,
        /// Text shown after the number, such as `"%"`.
        #[serde(default, skip_serializing_if = "String::is_empty")]
        suffix: String,
        /// Stepping past one end goes to the other.
        #[serde(default, skip_serializing_if = "is_false")]
        wrap: bool,
        /// An action run whenever the value changes.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        on_change: Option<String>,
    },
    /// A whole number on a track.
    Slider {
        /// The lowest value.
        #[serde(default, skip_serializing_if = "is_zero")]
        min: i64,
        /// The highest value.
        max: i64,
        /// The value at first.
        #[serde(default, skip_serializing_if = "is_zero")]
        value: i64,
        /// How much one step changes it.
        #[serde(default = "one", skip_serializing_if = "is_one")]
        step: i64,
        /// An action run whenever the value changes.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        on_change: Option<String>,
    },
    /// Progress towards `max`; set by code, not by the person.
    ProgressBar {
        /// The value that means done.
        #[serde(default = "hundred")]
        max: u64,
        /// The value at first.
        #[serde(default)]
        value: u64,
        /// Show the percentage.
        #[serde(default = "yes", skip_serializing_if = "is_true")]
        show_percent: bool,
    },
    /// Rows and columns with a header.
    Table {
        /// The columns, left to right.
        columns: Vec<ColumnSpec>,
        /// The rows shown at first; code usually fills them.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        rows: Vec<Vec<String>>,
        /// Show the header row.
        #[serde(default = "yes", skip_serializing_if = "is_true")]
        header: bool,
        /// Draw lines between the columns.
        #[serde(default, skip_serializing_if = "is_false")]
        separators: bool,
    },
    /// A titled box drawn around other controls. It only draws: the
    /// controls inside it are its siblings, listed after it.
    GroupBox {
        /// The title in its top edge.
        #[serde(default)]
        title: String,
    },
    /// Tabs over pages of controls; each page's controls are placed from
    /// the page's own corner.
    Tabs {
        /// The pages, left to right.
        pages: Vec<PageSpec>,
        /// The page shown at first.
        #[serde(default, skip_serializing_if = "is_usize_zero")]
        active: usize,
    },
    /// A text field that suggests matching items as you type.
    AutoComplete {
        /// The suggestions.
        items: Vec<String>,
        /// The text shown at first.
        #[serde(default, skip_serializing_if = "String::is_empty")]
        text: String,
        /// The most characters it takes.
        #[serde(default = "input_max_len", skip_serializing_if = "is_input_max_len")]
        max_len: usize,
        /// How many characters open the suggestions.
        #[serde(default = "min_chars", skip_serializing_if = "is_min_chars")]
        min_chars: usize,
        /// It may not be left blank.
        #[serde(default, skip_serializing_if = "is_false")]
        required: bool,
        /// Text that is not blank must be one of the items.
        #[serde(default, skip_serializing_if = "is_false")]
        require_match: bool,
    },
    /// Two lists side by side, to pick a subset of the items.
    DualList {
        /// The items: a string stands for itself, or `{"key", "label"}`.
        items: Vec<Item>,
        /// The keys chosen at first.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        chosen: Vec<String>,
        /// The titles over the two lists; "Available" and "Chosen" when
        /// absent.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        titles: Option<(String, String)>,
        /// At least this many must be chosen.
        #[serde(default, skip_serializing_if = "is_usize_zero")]
        min: usize,
        /// At most this many can be chosen.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        max: Option<usize>,
    },
    /// A tree of expandable nodes, one of which is selected (the first,
    /// unless `selected` says otherwise). Its value is the key of the
    /// selected node, or null for a tree with no nodes. Right and Left
    /// expand and collapse a node; Enter toggles it.
    Tree {
        /// The top-level nodes.
        nodes: Vec<TreeNode>,
        /// The key of the node selected at first.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        selected: Option<String>,
    },
    /// A rule drawn with line characters: a row when it is one row tall, a
    /// column when it is one column wide, and a rectangle with proper
    /// corners when it is larger than that.
    Line {
        /// The characters it is drawn with.
        #[serde(default, skip_serializing_if = "LineStyle::is_single")]
        style: LineStyle,
    },
    /// Where the dialog shows what is wrong when validation fails. Without
    /// one, the first error is shown in a message box.
    ErrorLine {},
}

#[allow(
    clippy::trivially_copy_pass_by_ref,
    reason = "serde's skip_serializing_if signature"
)]
fn is_usize_zero(n: &usize) -> bool {
    *n == 0
}

impl Kind {
    /// The type as written in JSON, such as `"radio_buttons"`.
    #[must_use]
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Label { .. } => "label",
            Self::StaticText { .. } => "static_text",
            Self::Button { .. } => "button",
            Self::Input { .. } => "input",
            Self::Memo { .. } => "memo",
            Self::CheckBox { .. } => "check_box",
            Self::CheckBoxes { .. } => "check_boxes",
            Self::RadioButtons { .. } => "radio_buttons",
            Self::ComboBox { .. } => "combo_box",
            Self::ListBox { .. } => "list_box",
            Self::Spinner { .. } => "spinner",
            Self::Slider { .. } => "slider",
            Self::ProgressBar { .. } => "progress_bar",
            Self::Table { .. } => "table",
            Self::GroupBox { .. } => "group_box",
            Self::Tabs { .. } => "tabs",
            Self::AutoComplete { .. } => "auto_complete",
            Self::DualList { .. } => "dual_list",
            Self::Line { .. } => "line",
            Self::Tree { .. } => "tree",
            Self::ErrorLine {} => "error_line",
        }
    }

    /// Whether the control has a value in the dialog's data.
    #[must_use]
    pub fn has_value(&self) -> bool {
        !matches!(
            self,
            Self::Label { .. }
                | Self::StaticText { .. }
                | Self::Button { .. }
                | Self::GroupBox { .. }
                | Self::Tabs { .. }
                | Self::Line { .. }
                | Self::ErrorLine {}
        )
    }

    /// The text a person reads on the control, if it has one: a label's or
    /// button's caption, a group box's title.
    #[must_use]
    pub fn caption(&self) -> Option<&str> {
        match self {
            Self::Label { text, .. }
            | Self::StaticText { text, .. }
            | Self::Button { text, .. }
            | Self::CheckBox { text, .. } => Some(text),
            Self::GroupBox { title } => Some(title),
            _ => None,
        }
    }
}

/// What an input line accepts as it is typed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum ValidatorSpec {
    /// A whole number from `min` to `max`.
    Range {
        /// The lowest value.
        min: i64,
        /// The highest value.
        max: i64,
    },
    /// Only these characters.
    Filter {
        /// Every character allowed.
        chars: String,
    },
    /// A picture: `#` digit, `@` letter, `&` upper-case letter, `!` any
    /// character; other characters are filled in.
    Picture {
        /// The picture, such as `"##/##/####"`.
        mask: String,
    },
    /// One of a list of values.
    Lookup {
        /// The values allowed.
        values: Vec<String>,
    },
    /// A validator your code registers under `name`
    /// ([`JsonDialog::validator`](super::JsonDialog::validator)).
    Custom {
        /// The name it is registered under.
        name: String,
    },
}

/// A table column.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ColumnSpec {
    /// The header.
    pub title: String,
    /// The width in columns.
    pub width: u16,
    /// `"left"` (the default) or `"right"`.
    #[serde(default, skip_serializing_if = "Align::is_left")]
    pub align: Align,
}

/// The characters a [`Kind::Line`] is drawn with.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LineStyle {
    /// `─ │ ┌ ┐ └ ┘`
    #[default]
    Single,
    /// `═ ║ ╔ ╗ ╚ ╝`
    Double,
    /// `━ ┃ ┏ ┓ ┗ ┛`
    Heavy,
    /// `─ │ ╭ ╮ ╰ ╯`
    Round,
    /// Solid blocks, `█`.
    Block,
}

impl LineStyle {
    /// Every style, in the order a designer lists them.
    pub const ALL: [Self; 5] = [
        Self::Single,
        Self::Double,
        Self::Heavy,
        Self::Round,
        Self::Block,
    ];

    /// The style as written in JSON.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Single => "single",
            Self::Double => "double",
            Self::Heavy => "heavy",
            Self::Round => "round",
            Self::Block => "block",
        }
    }

    #[allow(
        clippy::trivially_copy_pass_by_ref,
        reason = "serde's skip_serializing_if signature"
    )]
    fn is_single(&self) -> bool {
        *self == Self::Single
    }
}

/// How a table column aligns its cells.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Align {
    /// Flush left.
    #[default]
    Left,
    /// Flush right, for numbers.
    Right,
}

impl Align {
    #[allow(
        clippy::trivially_copy_pass_by_ref,
        reason = "serde's skip_serializing_if signature"
    )]
    fn is_left(&self) -> bool {
        *self == Self::Left
    }
}

/// A tab page.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PageSpec {
    /// The tab's title; a `~` marks its Alt hot key.
    pub title: String,
    /// The page's controls, placed from the page's corner.
    #[serde(default)]
    pub controls: Vec<Control>,
}

/// A dual list item: a string that stands for itself, or a key shown
/// with a label.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum Item {
    /// The key is also the label.
    Plain(String),
    /// A key, shown as `label`.
    Keyed {
        /// The value in the data.
        key: String,
        /// What the person sees.
        label: String,
    },
}

impl<'de> Deserialize<'de> for Item {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = Value::deserialize(deserializer)?;
        if let Value::String(s) = value {
            return Ok(Self::Plain(s));
        }
        let keyed = value.as_object().and_then(|m| {
            let key = m.get("key")?.as_str()?;
            let label = m.get("label")?.as_str()?;
            (m.len() == 2).then(|| Self::Keyed {
                key: key.to_string(),
                label: label.to_string(),
            })
        });
        keyed.ok_or_else(|| {
            D::Error::custom(format!(
                "an item is a string or {{\"key\": ..., \"label\": ...}}, not {value}"
            ))
        })
    }
}

impl Item {
    /// The key: the value in the dialog's data.
    #[must_use]
    pub fn key(&self) -> &str {
        match self {
            Self::Plain(key) | Self::Keyed { key, .. } => key,
        }
    }

    /// What the person sees.
    #[must_use]
    pub fn label(&self) -> &str {
        match self {
            Self::Plain(label) | Self::Keyed { label, .. } => label,
        }
    }
}

/// A node of a [`Kind::Tree`]: a string that stands for itself, or an
/// object with a `label`, and optionally a `key` (the label by default),
/// `children` and `expanded`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TreeNode {
    /// The value in the data; the label when none is given.
    pub key: Option<String>,
    /// What the person sees.
    pub label: String,
    /// The nodes below it.
    pub children: Vec<TreeNode>,
    /// Showing its children at first.
    pub expanded: bool,
}

impl TreeNode {
    /// A leaf whose key is its label.
    #[must_use]
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            ..Self::default()
        }
    }

    /// The key: the value in the dialog's data.
    #[must_use]
    pub fn key(&self) -> &str {
        self.key.as_deref().unwrap_or(&self.label)
    }

    /// Call `f` on every node of `nodes`, parents before children.
    pub fn each<'a>(nodes: &'a [Self], f: &mut impl FnMut(&'a Self)) {
        for node in nodes {
            f(node);
            Self::each(&node.children, f);
        }
    }

    /// The keys from a top-level node down to the node `key`.
    #[must_use]
    pub fn path<'a>(nodes: &'a [Self], key: &str) -> Option<Vec<&'a str>> {
        nodes.iter().find_map(|node| {
            if node.key() == key {
                Some(vec![node.key()])
            } else {
                let mut path = Self::path(&node.children, key)?;
                path.insert(0, node.key());
                Some(path)
            }
        })
    }
}

impl Serialize for TreeNode {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let key = self.key.as_ref().filter(|k| **k != self.label);
        if key.is_none() && self.children.is_empty() && !self.expanded {
            return serializer.serialize_str(&self.label);
        }
        let mut map = Map::new();
        if let Some(key) = key {
            map.insert("key".into(), Value::from(key.clone()));
        }
        map.insert("label".into(), Value::from(self.label.clone()));
        if !self.children.is_empty() {
            map.insert(
                "children".into(),
                serde_json::to_value(&self.children).map_err(serde::ser::Error::custom)?,
            );
        }
        if self.expanded {
            map.insert("expanded".into(), Value::Bool(true));
        }
        map.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for TreeNode {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = Value::deserialize(deserializer)?;
        let wrong = || {
            D::Error::custom(format!(
                "a tree node is a string or {{\"label\": ..., \"key\", \"children\", \"expanded\"}}, not {value}"
            ))
        };
        if let Value::String(label) = &value {
            return Ok(Self::new(label.clone()));
        }
        let map = value.as_object().ok_or_else(wrong)?;
        if map
            .keys()
            .any(|k| !["key", "label", "children", "expanded"].contains(&k.as_str()))
        {
            return Err(wrong());
        }
        let label = map.get("label").and_then(Value::as_str).ok_or_else(wrong)?;
        let key = match map.get("key") {
            None => None,
            Some(k) => Some(k.as_str().ok_or_else(wrong)?.to_string()),
        };
        let children = match map.get("children") {
            None => Vec::new(),
            Some(c) => serde_json::from_value(c.clone()).map_err(D::Error::custom)?,
        };
        let expanded = match map.get("expanded") {
            None => false,
            Some(e) => e.as_bool().ok_or_else(wrong)?,
        };
        Ok(Self {
            key,
            label: label.to_string(),
            children,
            expanded,
        })
    }
}

// ---- a control as one flat JSON object ------------------------------------

/// The keys every control has, before the keys of its kind.
const PLACE: [&str; 4] = ["x", "y", "w", "h"];

impl Serialize for Control {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let Value::Object(mut kind) =
            serde_json::to_value(&self.kind).map_err(serde::ser::Error::custom)?
        else {
            return Err(serde::ser::Error::custom("a control kind is an object"));
        };
        let mut map = Map::new();
        if let Some(t) = kind.remove("type") {
            map.insert("type".into(), t);
        }
        if !self.name.is_empty() {
            map.insert("name".into(), Value::from(self.name.clone()));
        }
        for (key, value) in PLACE.iter().zip([self.x, self.y, self.w, self.h]) {
            map.insert((*key).into(), Value::from(value));
        }
        if let Some(hint) = &self.hint {
            map.insert("hint".into(), Value::from(hint.clone()));
        }
        map.extend(kind);
        map.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Control {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let mut map = Map::<String, Value>::deserialize(deserializer)?;
        let name = match map.remove("name") {
            None => String::new(),
            Some(Value::String(name)) => name,
            Some(other) => {
                return Err(D::Error::custom(format!(
                    "a control's name is a string, not {other}"
                )));
            }
        };
        let type_name = map
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or("control")
            .to_string();
        let what = if name.is_empty() {
            type_name
        } else {
            format!("{type_name} \"{name}\"")
        };
        let mut place = [0i16; 4];
        for (slot, key) in place.iter_mut().zip(PLACE) {
            let value = map
                .remove(key)
                .ok_or_else(|| D::Error::custom(format!("{what}: missing `{key}`")))?;
            *slot = value
                .as_i64()
                .and_then(|n| i16::try_from(n).ok())
                .ok_or_else(|| {
                    D::Error::custom(format!("{what}: `{key}` is not a whole number: {value}"))
                })?;
        }
        let hint = match map.remove("hint") {
            None => None,
            Some(Value::String(hint)) => Some(hint),
            Some(other) => {
                return Err(D::Error::custom(format!(
                    "{what}: `hint` is a string, not {other}"
                )));
            }
        };
        let kind = Kind::deserialize(Value::Object(map))
            .map_err(|e| D::Error::custom(format!("{what}: {e}")))?;
        let [x, y, w, h] = place;
        Ok(Self {
            name,
            x,
            y,
            w,
            h,
            hint,
            kind,
        })
    }
}
