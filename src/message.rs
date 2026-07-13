use miniserde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Announce {
    alias: String,
    version: String,
    #[serde(rename = "deviceModel")]
    device_model: Option<String>,
    #[serde(rename = "deviceType")]
    device_type: Option<String>,
    fingerprint: String,
    port: usize,
    protocol: String,
    download: Option<bool>,
    announce: bool,
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
            announce: true,
        }
    }
}
