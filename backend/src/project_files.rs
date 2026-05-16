use std::{
    collections::HashMap,
    fs,
    path::{Component, Path, PathBuf},
};

use crate::error::ApiError;

const MAX_PROJECT_BYTES: usize = 1024 * 1024;

pub(crate) fn write_project_files(
    data_dir: &Path,
    project_id: &str,
    files: &HashMap<String, String>,
) -> Result<(), ApiError> {
    let project_dir = data_dir.join("projects").join(project_id);
    if project_dir.exists() {
        fs::remove_dir_all(&project_dir)
            .map_err(|_| ApiError::internal("clear project directory"))?;
    }
    fs::create_dir_all(&project_dir).map_err(|_| ApiError::internal("create project directory"))?;
    for (path, contents) in files {
        let safe_path = safe_relative_path(path)?;
        let target = project_dir.join(safe_path);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|_| ApiError::internal("create file directory"))?;
        }
        fs::write(target, contents).map_err(|_| ApiError::internal("write project file"))?;
    }
    Ok(())
}

pub(crate) fn read_project_files(
    data_dir: &Path,
    project_id: &str,
) -> Result<HashMap<String, String>, ApiError> {
    let project_dir = data_dir.join("projects").join(project_id);
    let mut files = HashMap::new();
    read_files_recursive(&project_dir, &project_dir, &mut files)?;
    Ok(files)
}

fn read_files_recursive(
    base: &Path,
    dir: &Path,
    files: &mut HashMap<String, String>,
) -> Result<(), ApiError> {
    if !dir.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(dir).map_err(|_| ApiError::internal("read project directory"))? {
        let entry = entry.map_err(|_| ApiError::internal("read project entry"))?;
        let path = entry.path();
        if path.is_dir() {
            read_files_recursive(base, &path, files)?;
        } else if path.is_file() {
            let rel = path
                .strip_prefix(base)
                .map_err(|_| ApiError::internal("project path"))?
                .to_string_lossy()
                .replace('\\', "/");
            let contents =
                fs::read_to_string(&path).map_err(|_| ApiError::internal("read project file"))?;
            files.insert(rel, contents);
        }
    }
    Ok(())
}

pub(crate) fn validate_files(files: &HashMap<String, String>) -> Result<(), ApiError> {
    if !files.contains_key("main.py") {
        return Err(ApiError::bad_request("project must include main.py"));
    }
    let total: usize = files
        .iter()
        .map(|(path, contents)| path.len() + contents.len())
        .sum();
    if total > MAX_PROJECT_BYTES {
        return Err(ApiError::bad_request("project is larger than 1 MB"));
    }
    for path in files.keys() {
        safe_relative_path(path)?;
    }
    Ok(())
}

fn safe_relative_path(path: &str) -> Result<PathBuf, ApiError> {
    let candidate = Path::new(path);
    if candidate.is_absolute() || path.is_empty() {
        return Err(ApiError::bad_request("invalid file path"));
    }
    let mut safe = PathBuf::new();
    for component in candidate.components() {
        match component {
            Component::Normal(part) => safe.push(part),
            _ => return Err(ApiError::bad_request("invalid file path")),
        }
    }
    Ok(safe)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_path_traversal() {
        assert!(safe_relative_path("../secret.txt").is_err());
        assert!(safe_relative_path("/secret.txt").is_err());
        assert!(safe_relative_path("nested/main.py").is_ok());
    }

    #[test]
    fn requires_main_py_and_size_limit() {
        let mut files = HashMap::new();
        files.insert("notes.txt".to_string(), "hello".to_string());
        assert!(validate_files(&files).is_err());

        files.insert("main.py".to_string(), "print(1)\n".to_string());
        assert!(validate_files(&files).is_ok());
    }

    #[test]
    fn writes_reads_and_replaces_project_files() -> anyhow::Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let project_id = "project-1";
        let mut files = HashMap::new();
        files.insert("main.py".to_string(), "print('hello')\n".to_string());
        files.insert(
            "lib/helpers.py".to_string(),
            "def answer():\n    return 42\n".to_string(),
        );

        write_project_files(temp_dir.path(), project_id, &files)?;
        assert_eq!(read_project_files(temp_dir.path(), project_id)?, files);

        let mut replacement = HashMap::new();
        replacement.insert("main.py".to_string(), "print('updated')\n".to_string());
        write_project_files(temp_dir.path(), project_id, &replacement)?;

        let saved = read_project_files(temp_dir.path(), project_id)?;
        assert_eq!(saved, replacement);
        assert!(!saved.contains_key("lib/helpers.py"));
        Ok(())
    }
}
