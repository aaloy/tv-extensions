// (C) 2026 - Enzo Lombardi

//! Where documents are saved: a directory, or a map in tests.

use std::collections::BTreeMap;

/// A place CSV files live. Paths are names at the disk's root.
pub trait Disk {
    /// A file's text.
    ///
    /// # Errors
    /// When it does not exist or the host refused.
    fn read(&self, path: &str) -> Result<String, String>;
    /// Stores a file.
    ///
    /// # Errors
    /// When the host refused (grant, quota).
    fn write(&mut self, path: &str, text: &str) -> Result<(), String>;
    /// The `.csv` files at the root, sorted.
    ///
    /// # Errors
    /// When the host refused.
    fn list(&self) -> Result<Vec<String>, String>;
}

/// An in-memory disk, for tests.
#[derive(Debug, Default)]
pub struct MemDisk(BTreeMap<String, String>);

impl Disk for MemDisk {
    fn read(&self, path: &str) -> Result<String, String> {
        self.0
            .get(path)
            .cloned()
            .ok_or_else(|| format!("no such file: /{path}"))
    }
    fn write(&mut self, path: &str, text: &str) -> Result<(), String> {
        self.0.insert(path.to_string(), text.to_string());
        Ok(())
    }
    fn list(&self) -> Result<Vec<String>, String> {
        Ok(self
            .0
            .keys()
            .filter(|k| k.ends_with(".csv"))
            .cloned()
            .collect())
    }
}

/// A directory on the local file system. Paths are file names at its root;
/// anything that is not a single ordinary path segment (empty, `.`, `..`,
/// a name with an embedded separator, or a name containing `:`) is refused,
/// so the editor can never reach outside `root`.
#[derive(Debug, Clone)]
pub struct FsDisk {
    root: std::path::PathBuf,
}

impl FsDisk {
    /// A disk over the directory `root`, which must already exist.
    pub fn new(root: impl Into<std::path::PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Resolves `path` to a file inside `root`, or refuses it.
    ///
    /// A name is accepted only when it is exactly one ordinary path segment:
    /// `std::path::Path::new(name).components()` must yield a single
    /// `Component::Normal` and nothing else. That alone rejects `..` and `.`
    /// (a `ParentDir`/`CurDir` component, never `Normal`) and anything with
    /// an embedded separator (more than one component, or on this OS a
    /// literal `\` that is only a separator on Windows but is refused here
    /// on every OS for consistency).
    ///
    /// It is not enough on its own, though: `PathBuf::join` treats a name
    /// such as `C:secret.csv` as having a root on drive `C` with no leading
    /// `\`, so on Windows `root.join("C:secret.csv")` discards `root` and
    /// resolves against the current directory of drive `C` instead —
    /// exactly the escape this method exists to prevent. And on every other
    /// OS, `:` is just an ordinary filename character, so
    /// `Path::new("C:secret.csv").components()` yields one `Normal`
    /// component and the component check alone would wave it through there.
    /// A name containing `:` is refused outright, so the refusal does not
    /// depend on which OS runs the check.
    fn file(&self, path: &str) -> Result<std::path::PathBuf, String> {
        let name = path.trim_start_matches('/');
        let is_single_normal_component = {
            let mut components = std::path::Path::new(name).components();
            matches!(components.next(), Some(std::path::Component::Normal(_)))
                && components.next().is_none()
        };
        if name.is_empty() || name.contains([':', '\\']) || !is_single_normal_component {
            return Err(format!("not a file name: {path}"));
        }
        Ok(self.root.join(name))
    }
}

impl Disk for FsDisk {
    fn read(&self, path: &str) -> Result<String, String> {
        std::fs::read_to_string(self.file(path)?).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                format!("no such file: /{}", path.trim_start_matches('/'))
            } else {
                format!("{path}: {e}")
            }
        })
    }
    fn write(&mut self, path: &str, text: &str) -> Result<(), String> {
        std::fs::write(self.file(path)?, text).map_err(|e| format!("{path}: {e}"))
    }
    fn list(&self) -> Result<Vec<String>, String> {
        let mut names: Vec<String> = std::fs::read_dir(&self.root)
            .map_err(|e| e.to_string())?
            .filter_map(Result::ok)
            .filter(|e| e.path().is_file())
            .filter_map(|e| e.file_name().into_string().ok())
            .filter(|n| {
                std::path::Path::new(n)
                    .extension()
                    .is_some_and(|e| e.eq_ignore_ascii_case("csv"))
            })
            .collect();
        names.sort();
        Ok(names)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mem_disk_round_trips_and_lists_only_csv() {
        let mut d = MemDisk::default();
        d.write("b.csv", "x").unwrap();
        d.write("a.csv", "y").unwrap();
        d.write("notes.txt", "z").unwrap();
        assert_eq!(d.read("a.csv").unwrap(), "y");
        assert_eq!(d.list().unwrap(), ["a.csv", "b.csv"]);
        assert!(d.read("missing.csv").is_err());
    }

    #[test]
    fn fs_disk_round_trips_and_lists_only_csv_files() {
        let dir = tempfile::tempdir().unwrap();
        let mut disk = FsDisk::new(dir.path());
        disk.write("b.csv", "x\n").unwrap();
        disk.write("a.csv", "y\n").unwrap();
        std::fs::write(dir.path().join("notes.txt"), "z").unwrap();
        std::fs::write(dir.path().join("UPPER.CSV"), "u\n").unwrap();
        assert_eq!(disk.read("a.csv").unwrap(), "y\n");
        assert_eq!(disk.list().unwrap(), vec!["UPPER.CSV", "a.csv", "b.csv"]);
    }

    #[test]
    fn fs_disk_reports_a_missing_file_like_mem_disk() {
        let dir = tempfile::tempdir().unwrap();
        let disk = FsDisk::new(dir.path());
        assert_eq!(
            disk.read("nope.csv").unwrap_err(),
            MemDisk::default().read("nope.csv").unwrap_err()
        );
    }

    #[test]
    fn fs_disk_refuses_paths_outside_its_root() {
        let dir = tempfile::tempdir().unwrap();
        let inner = dir.path().join("inner");
        std::fs::create_dir(&inner).unwrap();
        std::fs::write(dir.path().join("secret.csv"), "s").unwrap();
        let mut disk = FsDisk::new(&inner);
        for bad in [
            "../secret.csv",
            "..",
            ".",
            "./a.csv",
            "a/b.csv",
            "a\\b.csv",
            "C:x.csv",
            "",
        ] {
            assert!(disk.read(bad).is_err(), "read {bad:?}");
            assert!(disk.write(bad, "x").is_err(), "write {bad:?}");
        }
        assert_eq!(
            std::fs::read_to_string(dir.path().join("secret.csv")).unwrap(),
            "s"
        );
    }
}
