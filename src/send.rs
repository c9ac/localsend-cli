use crate::{Device, DynError, discover, protocol::*};
use miniserde::json;
use std::{collections::HashMap, fs::File, io::Read, path::PathBuf, time::Duration};
use ureq::Error;
use zfish::{
    Prompt,
    table::{Alignment, BoxStyle, Table},
};

pub fn send(
    files: Vec<PathBuf>,
    timeout: Duration,
    alias: &str,
    port: usize,
) -> Result<(), DynError> {
    // Interactive select device
    let device = select_device(timeout)?;

    // Build PrepareUpload
    let (prepare_upload, id_path) = build_prepare_upload(files, alias, port)?;
    let prepare_upload = json::to_string(&prepare_upload);

    let http_address = format!("http://{}:{}", device.address, device.info.port);
    let uri = format!("{}{}", http_address, PREPARE_UPLOAD_URI);
    let response = ureq::post(uri)
        .content_type("application/json")
        .send(&prepare_upload);

    // Handle rejection
    if let Err(Error::StatusCode(403)) = response {
        return Err("Request was rejected".into());
    }
    let response = response?;

    // Get session id and file tokens
    let mut body = String::new();
    response.into_body().as_reader().read_to_string(&mut body)?;
    let accept_upload: AcceptUpload = json::from_str(&body)?;

    // Upload file
    for (file_id, token) in accept_upload.files {
        let path = id_path.get(&file_id).ok_or("Known file id")?;
        let file_uri = build_upload_uri(&http_address, &accept_upload.session_id, &file_id, &token);
        let file = File::open(path)?;

        ureq::post(file_uri).send(file)?;
    }

    Ok(())
}

fn select_device(timeout: Duration) -> Result<Device, DynError> {
    // Discover through udp multicast
    let mut devices = discover(timeout)?;

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

fn build_prepare_upload(
    files: Vec<PathBuf>,
    alias: &str,
    port: usize,
) -> Result<(PrepareUpload, HashMap<String, PathBuf>), DynError> {
    let mut prepare_upload_files = HashMap::new();
    let mut id_path = HashMap::new();

    let info = Announce::new(alias, port);

    for file in files {
        let file_info = FileInfo::new(&file)?;
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
