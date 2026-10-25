// (C) 2026 - Antoni Aloy

//! Control templates: groups of controls kept to insert again, such as OK
//! and Cancel, or a label with its input. Each is a dialog spec just big
//! enough to hold its controls, with them at its corner.
//!
//! The samples come with the designer, built in, and cannot be renamed or
//! deleted. The person's own are files in the `controls` folder of the
//! templates folder, kept like the dialog templates.

use crate::templates;
use std::path::{Path, PathBuf};
use tv_extensions::dialog_spec::DialogSpec;

/// The samples, by file name and JSON.
const SAMPLES: [(&str, &str); 7] = [
    (
        "ok_cancel",
        include_str!("control_templates/ok_cancel.json"),
    ),
    (
        "ok_cancel_help",
        include_str!("control_templates/ok_cancel_help.json"),
    ),
    (
        "yes_no_cancel",
        include_str!("control_templates/yes_no_cancel.json"),
    ),
    (
        "labelled_input",
        include_str!("control_templates/labelled_input.json"),
    ),
    ("search", include_str!("control_templates/search.json")),
    ("sign_in", include_str!("control_templates/sign_in.json")),
    ("address", include_str!("control_templates/address.json")),
];

/// Where the person's control templates are kept: `controls` in the
/// templates folder.
pub fn folder() -> PathBuf {
    templates::folder().join("controls")
}

/// A control template: its name, the controls, and whether it is a sample.
#[derive(Debug, Clone)]
pub struct ControlTemplate {
    pub name: String,
    pub spec: DialogSpec,
    /// Built in: it cannot be renamed or deleted.
    pub sample: bool,
}

impl ControlTemplate {
    /// One line for a list: name, whose it is, and how many controls.
    pub fn summary(&self) -> String {
        let mut controls = 0;
        self.spec.visit(&mut |_| controls += 1);
        let whose = if self.sample { "sample" } else { "yours" };
        let name: String = self.name.chars().take(26).collect();
        format!("{name:<26} {whose:<7} {controls:>3}")
    }
}

/// The samples, then the templates in `dir`, by name. A file named like a
/// sample is left out: the sample keeps the name.
pub fn list(dir: &Path) -> Vec<ControlTemplate> {
    let mut all: Vec<ControlTemplate> = SAMPLES
        .iter()
        .filter_map(|&(_, json)| {
            let spec = DialogSpec::from_json(json).ok()?;
            Some(ControlTemplate {
                name: spec.title.clone(),
                spec,
                sample: true,
            })
        })
        .collect();
    for t in templates::list(dir) {
        if !is_sample(&t.name) {
            all.push(ControlTemplate {
                name: t.name,
                spec: t.spec,
                sample: false,
            });
        }
    }
    all
}

/// Whether `name` is taken by a sample, by its title or its file name.
pub fn is_sample(name: &str) -> bool {
    let stem = templates::file_stem(name).to_lowercase();
    SAMPLES.iter().any(|&(file, json)| {
        let title = DialogSpec::from_json(json)
            .map(|s| s.title)
            .unwrap_or_default();
        file == stem || templates::file_stem(&title).to_lowercase() == stem
    })
}

fn refuse_sample(name: &str) -> Result<(), String> {
    if is_sample(name) {
        Err(format!(
            "\"{}\" is a sample that comes with the designer: it cannot be changed. Pick another name.",
            name.trim()
        ))
    } else {
        Ok(())
    }
}

/// Whether the person has a control template called `name` in `dir`.
pub fn exists(dir: &Path, name: &str) -> bool {
    templates::exists(dir, name)
}

/// Save `spec` as the control template `name` in `dir`.
///
/// # Errors
///
/// The name is a sample's or has no usable letters, or the file cannot be
/// written.
pub fn save(dir: &Path, name: &str, spec: &DialogSpec) -> Result<PathBuf, String> {
    refuse_sample(name)?;
    templates::save(dir, name, spec)
}

/// Rename the control template `from` to `to`.
///
/// # Errors
///
/// Either name is a sample's, `to` is empty or taken, or the file cannot be
/// renamed.
pub fn rename(dir: &Path, from: &str, to: &str) -> Result<(), String> {
    refuse_sample(from)?;
    refuse_sample(to)?;
    templates::rename(dir, from, to)
}

/// Delete the control template `name`.
///
/// # Errors
///
/// It is a sample, or the file cannot be removed.
pub fn delete(dir: &Path, name: &str) -> Result<(), String> {
    refuse_sample(name)?;
    templates::delete(dir, name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tv_extensions::dialog_spec::JsonDialog;

    #[test]
    fn the_samples_build_and_come_first() {
        let tmp = tempfile::tempdir().unwrap();
        let all = list(tmp.path());
        assert_eq!(all.len(), SAMPLES.len());
        assert_eq!(all[0].name, "OK / Cancel");
        for t in &all {
            assert!(t.sample);
            assert!(!t.spec.controls.is_empty(), "{}", t.name);
            t.spec.check().unwrap_or_else(|e| panic!("{}: {e}", t.name));
            JsonDialog::new(&t.spec).unwrap_or_else(|e| panic!("{}: {e}", t.name));
        }
        // Every file in the folder is listed.
        let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("examples/dialog_designer/control_templates");
        assert_eq!(std::fs::read_dir(dir).unwrap().count(), SAMPLES.len());
    }

    #[test]
    fn yours_are_saved_renamed_and_deleted_after_the_samples() {
        let tmp = tempfile::tempdir().unwrap();
        let spec = DialogSpec::new("buttons", 20, 4);
        save(tmp.path(), "My buttons", &spec).unwrap();
        let all = list(tmp.path());
        let mine = all.last().unwrap();
        assert_eq!((mine.name.as_str(), mine.sample), ("My_buttons", false));

        rename(tmp.path(), "My_buttons", "Footer").unwrap();
        assert!(exists(tmp.path(), "Footer"));
        delete(tmp.path(), "Footer").unwrap();
        assert_eq!(list(tmp.path()).len(), SAMPLES.len());
    }

    #[test]
    fn a_sample_cannot_be_replaced_renamed_or_deleted() {
        let tmp = tempfile::tempdir().unwrap();
        let spec = DialogSpec::new("x", 20, 4);
        assert!(save(tmp.path(), "OK / Cancel", &spec).is_err());
        assert!(save(tmp.path(), "ok_cancel", &spec).is_err());
        assert!(rename(tmp.path(), "Search", "Mine").is_err());
        assert!(delete(tmp.path(), "Address").is_err());
        // A file put there by hand under a sample's name stays hidden.
        templates::save(tmp.path(), "search", &spec).unwrap();
        assert_eq!(list(tmp.path()).len(), SAMPLES.len());
    }
}
