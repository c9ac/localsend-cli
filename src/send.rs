use crate::{Device, DynError, discover, protocol::*};
use miniserde::json;
use std::{io::Read, path::PathBuf, time::Duration};
use ureq::Error;
use zfish::{
    Prompt,
    table::{Alignment, BoxStyle, Table},
};

pub fn send(files: Vec<PathBuf>, timeout: u64, alias: &str, port: usize) -> Result<(), DynError> {
    let device = select_device(timeout)?;

    let prepare_upload = PrepareUpload::new(files, alias, port)?;
    let prepare_upload = json::to_string(&prepare_upload);

    let uri = format!(
        "http://{}:{}{}",
        device.address, device.info.port, PREPARE_UPLOAD_URI
    );
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

    Ok(())
}

fn select_device(timeout: u64) -> Result<Device, DynError> {
    // Discover through udp multicast
    let mut devices = discover(Duration::from_secs(timeout))?;

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
