use miniserde::{Deserialize, Serialize};
use rs_machineid::MachineId;
use std::{collections::HashMap, path::PathBuf};

use crate::DynError;

pub const PREPARE_UPLOAD_URI: &str = "/api/localsend/v2/prepare-upload";
pub const UPLOAD_PREFFIX: &str = "/api/localsend/v2/upload?";

#[derive(Serialize, Deserialize, Clone)]
pub struct Announce {
    pub alias: String,
    pub version: String,
    #[serde(rename = "deviceModel")]
    pub device_model: Option<String>,
    #[serde(rename = "deviceType")]
    pub device_type: Option<String>,
    pub fingerprint: String,
    pub port: usize,
    pub protocol: String,
    pub download: Option<bool>,
    pub announce: Option<bool>,
}

#[derive(Serialize, Deserialize)]
pub struct PrepareUpload {
    pub info: Announce,
    pub files: HashMap<String, FileInfo>,
}

#[derive(Serialize, Deserialize)]
pub struct FileInfo {
    pub id: String,
    #[serde(rename = "fileName")]
    pub file_name: String,
    pub size: u64,
    #[serde(rename = "fileType")]
    pub file_type: String,
    pub sha256: Option<String>,
    pub preview: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct AcceptUpload {
    #[serde(rename = "sessionId")]
    pub session_id: String,
    pub files: HashMap<String, String>,
}

impl Announce {
    pub fn new(alias: &str, port: usize) -> Self {
        let fingerprint =
            MachineId::get_hashed("localsend-cli").unwrap_or("localsendDevice".into());

        Announce {
            alias: alias.into(),
            version: "2.0".into(),
            device_model: None,
            device_type: Some("headless".into()),
            fingerprint,
            port,
            protocol: "http".into(),
            download: Some(false),
            announce: Some(true),
        }
    }

    pub fn empty() -> Self {
        Announce {
            alias: "".into(),
            version: "".into(),
            device_model: None,
            device_type: None,
            fingerprint: "".into(),
            port: 0,
            protocol: "".into(),
            download: None,
            announce: None,
        }
    }
}

impl PrepareUpload {
    pub fn new(files: Vec<PathBuf>, alias: &str, port: usize) -> Result<Self, DynError> {
        let info = Announce::new(alias, port);

        let files: HashMap<String, FileInfo> = files
            .into_iter()
            .map(|path| {
                let file_info = FileInfo::new(&path)?;
                Ok((file_info.id.clone(), file_info))
            })
            .collect::<Result<_, DynError>>()?;

        Ok(Self { info, files })
    }

    pub fn empty() -> Self {
        PrepareUpload {
            info: Announce::empty(),
            files: HashMap::new(),
        }
    }
}

impl FileInfo {
    pub fn new(file: &PathBuf) -> Result<Self, DynError> {
        if !file.exists() {
            return Err(format!("File {} was not found", file.display()).into());
        }

        let file_name = file
            .file_name()
            .and_then(|os| os.to_str())
            .map(String::from)
            .unwrap_or_else(|| "".to_string());

        let file_type = match infer::get_from_path(file)? {
            Some(t) => t.mime_type(),
            None => mime_guess::from_path(file)
                .first_raw()
                .unwrap_or("application/octet-stream"),
        }
        .to_string();

        let sha256 = rune_sha256::hash_file(file)?;

        let id: String = sha256.chars().take(8).collect();

        Ok(Self {
            id,
            file_name,
            size: file.metadata()?.len(),
            file_type,
            sha256: Some(sha256),
            preview: None,
        })
    }
}
