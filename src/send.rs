use crate::{DynError, device::*, http::*, protocol::*};
use miniserde::json;
use smol::fs::File;
use std::{collections::HashMap, path::PathBuf, time::Duration};
use zfish::{
    Prompt,
    table::{Alignment, BoxStyle, Table},
};

pub async fn send(
    files: Vec<PathBuf>,
    timeout: u64,
    alias: &str,
    port: usize,
) -> Result<(), DynError> {
    // Interactive select device
    let device = select_device(timeout).await?;

    // Build PrepareUpload
    let (prepare_upload, id_path) = build_prepare_upload(files, alias, port).await?;
    let prepare_upload = json::to_string(&prepare_upload).into_bytes();

    // Send upload request
    let base_url = format!("{}:{}", device.address, device.info.port);
    let accept_upload = post(&base_url, PREPARE_UPLOAD_URI, &prepare_upload).await?;
    let accept_upload = String::from_utf8_lossy(&accept_upload);

    // Parse session id and file tokens
    let accept_upload: AcceptUpload = json::from_str(&accept_upload)?;

    // Upload file
    for (file_id, token) in accept_upload.files {
        let path = id_path.get(&file_id).ok_or("Unknown file id")?;
        let file_uri = build_upload_path(&accept_upload.session_id, &file_id, &token);
        let mut file = File::open(path).await?;

        upload_file(&base_url, &file_uri, &mut file).await?;
    }

    Ok(())
}

async fn select_device(timeout: u64) -> Result<Device, DynError> {
    // Discover through udp multicast
    let mut devices = discover(Duration::from_secs(timeout)).await?;

    // Draw table and let user select
    let mut table = Table::new(vec!["", "Alias", "Type"]);
    table.set_box_style(BoxStyle::Rounded);
    table
        .set_column_alignment(1, Alignment::Center)
        .set_column_alignment(2, Alignment::Center);

    for (n, device) in devices.iter().enumerate() {
        table.add_row(vec![
            &format!("{}", n + 1),
            &device.info.alias,
            device.info.device_type.as_deref().unwrap_or(""),
        ]);
    }

    table.print();

    let select: usize = Prompt::text("Choose one device (input index):")?.parse()?;

    if select > devices.len() || select < 1 {
        return Err("Device index out of range".into());
    }

    // By index
    let device = devices.swap_remove(select - 1); // index will never be out of range
    Ok(device)
}

async fn build_prepare_upload(
    files: Vec<PathBuf>,
    alias: &str,
    port: usize,
) -> Result<(PrepareUpload, HashMap<String, PathBuf>), DynError> {
    let mut prepare_upload_files = HashMap::new();
    let mut id_path = HashMap::new();

    let info = Announce::new(alias, port);

    for file in files {
        let file_info = FileInfo::new(&file).await?;
        let id = file_info.id.clone();
        id_path.insert(id.clone(), file);
        prepare_upload_files.insert(id, file_info);
    }

    Ok((
        PrepareUpload {
            info,
            files: prepare_upload_files,
        },
        id_path,
    ))
}
