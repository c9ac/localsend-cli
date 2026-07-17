use crate::{Announce, DynError, PrepareUpload, convert_storage_unit};
use miniserde::json;
use std::{
    net::{Ipv4Addr, UdpSocket},
    thread,
    time::Duration,
};
use tiny_http::{Method, Request, Server};
use zfish::{
    Prompt,
    table::{Alignment, BoxStyle, Table},
};

pub fn receive(alias: &str, port: usize) -> Result<(), DynError> {
    let device = Announce::build(alias, port);

    announce(&device)?;

    let server = Server::http(format!("0.0.0.0:{}", port))?;

    for mut request in server.incoming_requests() {
        let mut body = String::new();
        request.as_reader().read_to_string(&mut body)?;

        if let Some(prepare_upload) = handle_request(&request, &body)? {
            let _is_receive = confirm_upload(&prepare_upload)?;
        }
    }

    Ok(())
}

fn announce(device: &Announce) -> Result<(), DynError> {
    let multicast_addr = Ipv4Addr::new(224, 0, 0, 167);
    let socket = UdpSocket::bind("0.0.0.0:53318")?;
    socket.join_multicast_v4(&multicast_addr, &Ipv4Addr::UNSPECIFIED)?;

    let announce = json::to_string(device).into_bytes();

    thread::spawn(move || {
        loop {
            let _ = socket.send_to(&announce, "224.0.0.167:53317");
            thread::sleep(Duration::from_secs(3));
        }
    });

    Ok(())
}

fn handle_request(request: &Request, body: &str) -> Result<Option<PrepareUpload>, DynError> {
    if !(request.url() == "/api/localsend/v2/prepare-upload" && request.method() == &Method::Post) {
        return Ok(None);
    }

    let prepare_upload: PrepareUpload = json::from_str(body)?;
    Ok(Some(prepare_upload))
}

fn confirm_upload(prepare_upload: &PrepareUpload) -> Result<bool, DynError> {
    let files = &prepare_upload.files;
    let alias = &prepare_upload.info.alias;

    let mut table = Table::new(vec!["", "File Name", "Size", "File Type", "From"]);

    table
        .set_box_style(BoxStyle::Rounded)
        .set_column_alignment(0, Alignment::Center)
        .set_column_alignment(1, Alignment::Center)
        .set_column_alignment(2, Alignment::Center);

    for (num, file_info) in (1..).zip(files.values()) {
        table.add_row(vec![
            &format!("{}", num),
            &file_info.file_name,
            &convert_storage_unit(file_info.size),
            &file_info.file_type,
            alias,
        ]);
    }

    table.print();

    Ok(Prompt::confirm("Receive?", true)?)
}
