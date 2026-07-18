use miniserde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize)]
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
    pub size: usize,
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
    pub fn build(alias: &str, port: usize) -> Self {
        Announce {
            alias: alias.into(),
            version: "2.0".into(),
            device_model: None,
            device_type: Some("headless".into()),
            fingerprint: "tmp_fingerprint".into(),
            port,
            protocol: "http".into(),
            download: Some(false),
            announce: Some(true),
        }
    }
}

impl PrepareUpload {
    pub fn new() -> Self {
        let info = Announce::build("", 53317);

        PrepareUpload {
            info,
            files: HashMap::new(),
        }
    }
}
