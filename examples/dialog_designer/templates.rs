// (C) 2026 - Antoni Aloy

//! Templates: dialog specs kept in a folder, to start a new dialog from.
//! They are ordinary dialog JSON files, so a template can be opened,
//! edited and saved like any other dialog.

use std::path::{Path, PathBuf};
use tv_extensions::dialog_spec::DialogSpec;

/// The environment variable that names the templates folder.
const FOLDER_VAR: &str = "DIALOG_TEMPLATES";

/// The templates folder: `$DIALOG_TEMPLATES`, or `templates` in the current
/// directory.
pub fn folder() -> PathBuf {
    std::env::var_os(FOLDER_VAR).map_or_else(|| PathBuf::from("templates"), PathBuf::from)
}

/// A template: the file it is in and the dialog it holds.
#[derive(Debug, Clone)]
pub struct Template {
    /// The file name without `.json`: how the template is called.
    pub name: String,
    pub spec: DialogSpec,
}

impl Template {
    /// One line for a list: name, the dialog's title, its size and how many
    /// controls it has.
    pub fn summary(&self) -> String {
        let mut controls = 0;
        self.spec.visit(&mut |_| controls += 1);
        format!(
            "{:<18} {:<22} {:>3}x{:<3} {:>3}",
            clip(&self.name, 18),
            clip(&self.spec.title, 22),
            self.spec.width,
            self.spec.height,
            controls
        )
    }
}

fn clip(text: &str, width: usize) -> String {
    text.chars().take(width).collect()
}

/// A file name for a template called `name`: letters, digits, `-` and `_`;
/// anything else becomes `_`. Empty if nothing is left.
pub fn file_stem(name: &str) -> String {
    let stem: String = name
        .trim()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    stem.trim_matches('_').to_string()
}

fn path_of(dir: &Path, name: &str) -> PathBuf {
    dir.join(format!("{}.json", file_stem(name)))
}

/// The templates in `dir`, by name. Files that are not valid dialogs are
/// left out; a missing folder has none.
pub fn list(dir: &Path) -> Vec<Template> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut found: Vec<Template> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "json"))
        .filter_map(|path| {
            let name = path.file_stem()?.to_string_lossy().into_owned();
            let spec = DialogSpec::load(&path).ok()?;
            Some(Template { name, spec })
        })
        .collect();
    found.sort_by_key(|t| t.name.to_lowercase());
    found
}

/// Whether a template called `name` exists in `dir`.
pub fn exists(dir: &Path, name: &str) -> bool {
    path_of(dir, name).exists()
}

/// Save `spec` as the template `name`, making the folder if need be.
///
/// # Errors
///
/// The name has no usable letters, or the file cannot be written.
pub fn save(dir: &Path, name: &str, spec: &DialogSpec) -> Result<PathBuf, String> {
    if file_stem(name).is_empty() {
        return Err("A template needs a name.".into());
    }
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let path = path_of(dir, name);
    spec.save(&path).map_err(|e| e.to_string())?;
    Ok(path)
}

/// Rename the template `from` to `to`.
///
/// # Errors
///
/// `to` is empty or taken, or the file cannot be renamed.
pub fn rename(dir: &Path, from: &str, to: &str) -> Result<(), String> {
    if file_stem(to).is_empty() {
        return Err("A template needs a name.".into());
    }
    let (old, new) = (path_of(dir, from), path_of(dir, to));
    if old == new {
        return Ok(());
    }
    if new.exists() {
        return Err(format!("There is a template called \"{to}\" already."));
    }
    std::fs::rename(&old, &new).map_err(|e| e.to_string())
}

/// Delete the template `name`.
///
/// # Errors
///
/// The file cannot be removed.
pub fn delete(dir: &Path, name: &str) -> Result<(), String> {
    std::fs::remove_file(path_of(dir, name)).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tv_extensions::dialog_spec::JsonDialog;

    fn spec(title: &str) -> DialogSpec {
        DialogSpec::new(title, 40, 10)
    }

    #[test]
    fn templates_are_saved_listed_renamed_and_deleted() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("templates");
        assert!(list(&dir).is_empty(), "a missing folder has none");

        save(&dir, "My form", &spec("Mine")).unwrap();
        save(&dir, "another", &spec("Other")).unwrap();
        let names: Vec<String> = list(&dir).into_iter().map(|t| t.name).collect();
        assert_eq!(names, ["another", "My_form"]);
        assert!(exists(&dir, "My form"));

        rename(&dir, "another", "third").unwrap();
        assert!(rename(&dir, "third", "My form").is_err());
        delete(&dir, "My form").unwrap();
        let left = list(&dir);
        assert_eq!(left.len(), 1);
        assert_eq!(left[0].name, "third");
        assert_eq!(left[0].spec.title, "Other");
        assert!(save(&dir, "  ?? ", &spec("x")).is_err());
    }

    #[test]
    fn a_file_that_is_not_a_dialog_is_not_a_template() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("junk.json"), "{ not a dialog").unwrap();
        std::fs::write(tmp.path().join("notes.txt"), "hello").unwrap();
        assert!(list(tmp.path()).is_empty());
    }

    /// The templates that ship with the designer all load and build.
    #[test]
    fn the_sample_templates_build() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("templates");
        let templates = list(&dir);
        assert!(templates.len() >= 8, "found {}", templates.len());
        for t in templates {
            t.spec.check().unwrap_or_else(|e| panic!("{}: {e}", t.name));
            JsonDialog::new(&t.spec).unwrap_or_else(|e| panic!("{}: {e}", t.name));
        }
        // Every JSON file in the folder is a valid template (`controls`, the
        // control templates' folder, may be there too).
        let files = std::fs::read_dir(&dir)
            .unwrap()
            .filter(|e| {
                e.as_ref()
                    .is_ok_and(|e| e.path().extension().is_some_and(|x| x == "json"))
            })
            .count();
        assert_eq!(files, list(&dir).len(), "a sample is not a valid dialog");
    }
}
