use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

use crate::detect::has_supported_extension;

fn is_hidden(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with('.'))
}

fn children(directory: &Path) -> Vec<PathBuf> {
    std::fs::read_dir(directory)
        .map(|entries| entries.flatten().map(|entry| entry.path()).collect())
        .unwrap_or_default()
}

fn expand(path: &Path) -> Vec<PathBuf> {
    if path.is_dir() {
        children(path)
            .iter()
            .flat_map(|child| expand_inside_directory(child))
            .collect()
    } else {
        vec![path.to_path_buf()]
    }
}

fn expand_inside_directory(path: &Path) -> Vec<PathBuf> {
    if is_hidden(path) {
        Vec::new()
    } else if path.is_dir() {
        expand(path)
    } else if has_supported_extension(path) {
        vec![path.to_path_buf()]
    } else {
        Vec::new()
    }
}

/// Files given explicitly are kept as they are (so unsupported ones are reported to the user);
/// folders are searched recursively for supported files, skipping hidden entries.
pub fn collect_files(paths: &[PathBuf]) -> Vec<PathBuf> {
    paths
        .iter()
        .flat_map(|path| expand(path))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn touch(root: &Path, relative: &str) -> PathBuf {
        let path = root.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, b"x").unwrap();
        path
    }

    #[test]
    fn folders_are_searched_recursively_for_supported_files() {
        let dir = tempfile::tempdir().unwrap();
        let a = touch(dir.path(), "lights/DSCF0001.RAF");
        let b = touch(dir.path(), "lights/sub/DSCF0002.RAF");
        touch(dir.path(), "lights/notes.txt");
        touch(dir.path(), "lights/.DS_Store");
        touch(dir.path(), ".hidden/DSCF0003.RAF");
        let found = collect_files(&[dir.path().to_path_buf()]);
        assert_eq!(found, vec![a, b]);
    }

    #[test]
    fn explicit_files_are_kept_even_if_unsupported() {
        let dir = tempfile::tempdir().unwrap();
        let text = touch(dir.path(), "readme.txt");
        assert_eq!(collect_files(std::slice::from_ref(&text)), vec![text]);
    }

    #[test]
    fn duplicates_are_removed_and_results_sorted() {
        let dir = tempfile::tempdir().unwrap();
        let a = touch(dir.path(), "b/2.fits");
        let b = touch(dir.path(), "a/1.fits");
        let found = collect_files(&[dir.path().to_path_buf(), a.clone()]);
        assert_eq!(found, vec![b, a]);
    }
}
