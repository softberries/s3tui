use std::fs;
use std::io::{self, BufRead};
use std::path::{Path, PathBuf};
use anyhow::bail;
use directories::UserDirs;

#[derive(Debug, Clone, Default)]
pub struct FileCredential {
    pub name: String,
    pub access_key: String,
    pub secret_key: String,
    pub selected: bool,
}

pub fn load_credentials() -> anyhow::Result<Vec<FileCredential>> {
    let path = get_credentials_dir()?;
    load_credentials_from_dir(path.as_path())
}

fn load_credentials_from_dir(dir_path: &Path) -> anyhow::Result<Vec<FileCredential>> {
    let mut credentials = Vec::new();
    let mut selected = true;
    for entry in fs::read_dir(dir_path)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            let (access_key, secret_key) = parse_credential_file(&path)?;

            credentials.push(FileCredential {
                name,
                access_key,
                secret_key,
                selected,
            });
            selected = false; // Only the first entry is selected
        }
    }

    Ok(credentials)
}

fn get_credentials_dir() -> anyhow::Result<PathBuf> {
    let user_dirs = UserDirs::new().ok_or(io::Error::new(io::ErrorKind::NotFound, "Cannot find home directory"))?;
    let home_dir = user_dirs.home_dir();
    let creds_dir = home_dir.join(".s3tui/creds");

    Ok(creds_dir)
}

fn parse_credential_file(path: &Path) -> anyhow::Result<(String, String)> {
    let file = fs::File::open(path)?;
    let reader = io::BufReader::new(file);
    let mut access_key = String::new();
    let mut secret_key = String::new();

    for line in reader.lines() {
        let line = line?;
        if line.starts_with("access_key=") {
            access_key = line["access_key=".len()..].trim().to_string();
        } else if line.starts_with("secret_key=") {
            secret_key = line["secret_key=".len()..].trim().to_string();
        }
    }

    if access_key.is_empty() || secret_key.is_empty() {
        bail!("Missing access_key or secret_key in file: {:?}", path);
    }

    Ok((access_key, secret_key))
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
        Ok(())
    }

    #[test]
    fn test_parse_credential_file() {
        let dir = tempdir().unwrap();
        setup_test_credentials(dir.path(), "cred1").unwrap();

        let file_path = dir.path().join("cred1");
        let (access_key, secret_key) = parse_credential_file(&file_path).unwrap();

        assert_eq!(access_key, "AKIAIOSFODNN7EXAMPLE");
        assert_eq!(secret_key, "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY");
    }

    #[test]
    fn test_load_credentials_no_files() {
        let dir = tempdir().unwrap();
        let creds = load_credentials_from_dir(dir.path()).unwrap();

        assert!(creds.is_empty());
    }

    #[test]
    fn test_load_credentials_with_files() {
        let dir = tempdir().unwrap();
        setup_test_credentials(dir.path(),"cred1").unwrap();

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
        credentials.iter()
            .filter(|cred| cred.selected)
            .count()
    }

}
