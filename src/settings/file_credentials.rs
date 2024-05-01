use std::fs;
use std::io::{self, BufRead};
use std::path::{Path, PathBuf};
use anyhow::bail;
use directories::UserDirs;

#[derive(Debug, Clone)]
pub struct FileCredential {
    pub name: String,
    pub access_key: String,
    pub secret_key: String,
    pub selected: bool
}

pub fn load_credentials() -> anyhow::Result<Vec<FileCredential>> {
    let path = get_credentials_dir()?;
    let mut credentials = Vec::new();
    let mut selected = true;
    for entry in fs::read_dir(path)? {
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
            //only the first entry will be selected by default
            selected = false;
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