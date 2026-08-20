use crate::DynError;
use miniserde::{Deserialize, Serialize};
use rs_machineid::MachineId;
use sha2::{Digest, Sha256};
use smol::{fs::File, io::AsyncReadExt};
use std::{collections::HashMap, path::PathBuf};

pub const PREPARE_UPLOAD_URI: &str = "/api/localsend/v2/prepare-upload";

pub const UPLOAD_PREFFIX: &str = "/api/localsend/v2/upload?";
pub fn build_upload_path(session_id: &str, file_id: &str, token: &str) -> String {
    format!(
        "{}sessionId={}&fileId={}&token={}",
        UPLOAD_PREFFIX, session_id, file_id, token
    )
}

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

#[derive(Serialize, Deserialize, Clone)]
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

impl Default for PrepareUpload {
    fn default() -> Self {
        PrepareUpload {
            info: Announce::empty(),
            files: HashMap::new(),
        }
    }
}

impl FileInfo {
    pub async fn new(file: &PathBuf) -> Result<Self, DynError> {
        let file_name = file
            .file_name()
            .and_then(|os| os.to_str())
            .map(String::from)
            .unwrap_or_else(|| "file".to_string());

        let file_type = match infer::get_from_path(file)? {
            Some(t) => t.mime_type(),
            None => mime_guess::from_path(file)
                .first_raw()
                .unwrap_or("application/octet-stream"),
        }
        .to_string();

        let sha256 = hash_file(file).await?;

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

async fn hash_file(file: &PathBuf) -> Result<String, DynError> {
    let mut file = File::open(file).await?;
    let mut hasher = Sha256::new();

    let mut buf = [0u8; 1024 * 8];
    loop {
        let n = file.read(&mut buf).await?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }

    let hash = hasher.finalize();
    Ok(hex::encode(hash))
}
