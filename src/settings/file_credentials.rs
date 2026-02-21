use crate::utils::get_data_dir;
use color_eyre::{eyre, Report};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, BufRead};
use std::path::{Path, PathBuf};

/// Representation of the minio stored in your configuration
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct FileCredential {
    pub name: String,
    pub access_key: String,
    pub secret_key: String,
    pub default_region: String,
    pub endpoint_url: Option<String>,
    pub force_path_style: bool,
    pub selected: bool,
}

impl FileCredential {
    fn try_parse_file(path: &Path, selected: bool) -> eyre::Result<Self> {
        let file = fs::File::open(path)?;
        let reader = io::BufReader::new(file);
        let mut access_key = String::new();
        let mut secret_key = String::new();
        let mut default_region = String::new();
        let mut endpoint_url = None;
        let mut force_path_style = false;
        let name = path.file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "default".to_string());

        for line in reader.lines() {
            let line = line?;
            if let Some(stripped) = line.strip_prefix("access_key=") {
                access_key = stripped.trim().to_string()
            } else if let Some(stripped) = line.strip_prefix("secret_key=") {
                secret_key = stripped.trim().to_string()
            } else if let Some(stripped) = line.strip_prefix("default_region=") {
                default_region = stripped.trim().to_string()
            } else if let Some(stripped) = line.strip_prefix("endpoint_url=") {
                endpoint_url = Some(stripped.trim().to_string())
            } else if let Some(stripped) = line.strip_prefix("force_path_style=") {
                force_path_style = stripped.trim().parse().expect("configuration param [force_path_style] is not a valid boolean (true/false)");
            }
        }

        if access_key.is_empty() || secret_key.is_empty() || default_region.is_empty() {
            panic!(
                "Missing access_key/secret_key/default_region in file: {:?}",
                path
            );
        }

        Ok(Self {
            name,
            access_key,
            secret_key,
            default_region,
            endpoint_url,
            force_path_style,
            selected,
        })
    }
}

pub fn load_credentials(creds_file: Option<PathBuf>) -> eyre::Result<Vec<FileCredential>> {
    if let Some(path) = creds_file {
        load_credentials_from_file(path.as_path())
    } else {
        let path = get_data_dir().join("creds");
        load_credentials_from_dir(path.as_path())
    }
}

fn load_credentials_from_file(dir_path: &Path) -> eyre::Result<Vec<FileCredential>> {
    Ok(vec![FileCredential::try_parse_file(dir_path, true)?])
}

fn load_credentials_from_dir(dir_path: &Path) -> eyre::Result<Vec<FileCredential>> {
    let mut credentials = Vec::new();
    let mut selected = true;
    for entry in fs::read_dir(dir_path)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            credentials.push(FileCredential::try_parse_file(&path, selected)?);
            selected = false; // Only the first entry is selected
        }
    }

    if credentials.is_empty() {
        Err(Report::msg("Missing minio in your data creds folder"))
    } else {
        Ok(credentials)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    fn setup_test_credentials(dir: &Path, file_name: &str) -> io::Result<()> {
        let file_path = dir.join(file_name);
        let mut file = fs::File::create(file_path)?;
        writeln!(file, "access_key=AKIAIOSFODNN7EXAMPLE")?;
        writeln!(file, "secret_key=wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY")?;
        writeln!(file, "default_region=eu-north-1")?;
        Ok(())
    }

    #[test]
    fn test_parse_credential_file() {
        let dir = tempdir().unwrap();
        setup_test_credentials(dir.path(), "cred1").unwrap();

        let file_path = dir.path().join("cred1");
        let credentials = FileCredential::try_parse_file(&file_path, true).unwrap();

        assert_eq!(credentials.access_key, "AKIAIOSFODNN7EXAMPLE");
        assert_eq!(credentials.secret_key, "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY");
        assert_eq!(credentials.default_region, "eu-north-1");
        assert_eq!(credentials.endpoint_url, None);
        assert_eq!(credentials.force_path_style, false);
    }

    #[test]
    fn test_load_credentials_no_files() {
        let dir = tempdir().unwrap();
        let res = load_credentials_from_dir(dir.path());
        assert_eq!(
            res.err().unwrap().to_string(),
            Report::msg("Missing minio in your data creds folder").to_string()
        );
    }

    #[test]
    fn test_load_credentials_with_files() {
        let dir = tempdir().unwrap();
        setup_test_credentials(dir.path(), "cred1").unwrap();

        let creds = load_credentials_from_dir(dir.path()).unwrap();

        assert_eq!(creds.len(), 1);
        assert_eq!(creds[0].name, "cred1");
        assert!(creds[0].selected);
    }

    #[test]
    fn test_load_credentials_with_files_only_one_set_as_selected() {
        let dir = tempdir().unwrap();
        setup_test_credentials(dir.path(), "cred1").unwrap();
        setup_test_credentials(dir.path(), "cred2").unwrap();
        setup_test_credentials(dir.path(), "cred3").unwrap();

        let creds = load_credentials_from_dir(dir.path()).unwrap();
        let selected_count = count_selected_credentials(&creds);
        assert_eq!(selected_count, 1);
        assert_eq!(creds.len(), 3);
    }

    fn count_selected_credentials(credentials: &[FileCredential]) -> usize {
        credentials.iter().filter(|cred| cred.selected).count()
    }
}
