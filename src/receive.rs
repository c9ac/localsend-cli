use crate::{AcceptUpload, Announce, DynError, PrepareUpload, convert_storage_unit};
use miniserde::json;
use std::{
    collections::HashMap,
    net::{Ipv4Addr, UdpSocket},
    thread,
    time::Duration,
};
use tiny_http::{Method, Request, Response, Server, StatusCode};
use zfish::{
    Prompt,
    table::{BoxStyle, Table},
};

pub fn receive(alias: &str, port: usize) -> Result<(), DynError> {
    let device = Announce::build(alias, port);

    announce(&device)?;

    let server = Server::http(format!("0.0.0.0:{}", port))?;

    for mut request in server.incoming_requests() {
        let mut body = String::new();
        request.as_reader().read_to_string(&mut body)?;

        // Listen to new urls if prepare upload request occur
        match handle_request(&request, &body) {
            Ok(Some(prepare_upload)) => {
                match respond_upload(&prepare_upload) {
                    Ok(files) => request.respond(Response::from_string(files))?,
                    Err(e) => {
                        if e.to_string() == "reject" {
                            request.respond(Response::empty(StatusCode::from(403)))?;
                        } else {
                            request.respond(Response::empty(StatusCode::from(500)))?;
                        }
                    }
                };
            }
            Err(_) => {
                request.respond(Response::empty(StatusCode::from(400)))?;
                continue;
            }
            _ => (),
        };
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

    table.set_box_style(BoxStyle::Rounded);

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

fn respond_upload(prepare_upload: &PrepareUpload) -> Result<String, DynError> {
    let is_receive = confirm_upload(prepare_upload)?;

    let mut files = HashMap::new();
    for file in prepare_upload.files.values() {
        files.insert(file.id.clone(), file.id.clone()); // use files id as token
    }

    let accept_upload = AcceptUpload {
        session_id: "session0".into(),
        files,
    };
    let accept_upload = json::to_string(&accept_upload);

    if is_receive {
        Ok(accept_upload)
    } else {
        Err("reject".into())
    }
}
