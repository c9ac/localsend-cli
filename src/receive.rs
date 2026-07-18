use crate::{AcceptUpload, Announce, DynError, PrepareUpload, convert_storage_unit};
use miniserde::json;
use std::{
    collections::HashMap,
    fs::File,
    io::copy,
    net::{Ipv4Addr, UdpSocket},
    thread,
    time::Duration,
};
use tiny_http::{Method, Response, Server, StatusCode};
use zfish::{
    Alignment, Prompt, Terminal,
    table::{BoxStyle, Table},
};

pub fn receive(alias: &str, port: usize) -> Result<(), DynError> {
    Terminal::clear_screen()?;
    Terminal::move_cursor(0, 0)?;

    let device = Announce::build(alias, port);

    announce(&device)?;

    let server = Server::http(format!("0.0.0.0:{}", port))?;
    let mut prepare_upload = PrepareUpload::new();
    let mut unknown_count = 0;
    let mut file_status = HashMap::new();

    for mut request in server.incoming_requests() {
        // Listen to new urls if prepare upload request occur
        if request.url() == "/api/localsend/v2/prepare-upload" && request.method() == &Method::Post
        {
            unknown_count = 0;
            file_status.clear();

            let mut body = String::new();
            request.as_reader().read_to_string(&mut body)?;

            prepare_upload = json::from_str(&body)?;
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
        // Listen to file upload
        else if request.url().starts_with("/api/localsend/v2/upload?")
            && request.method() == &Method::Post
        {
            // Get file name by file id
            let file_id = request
                .url()
                .split("&")
                .find(|s| s.starts_with("fileId="))
                .and_then(|s| s.get(7..))
                .map(|s| s.to_string());

            if file_id.is_none() {
                request.respond(Response::empty(StatusCode(500)))?;
                continue;
            }
            let file_id = file_id.unwrap();

            // Write to file
            let file_path = match prepare_upload.files.get(&file_id) {
                Some(file_info) => &file_info.file_name,
                None => {
                    unknown_count += 1;
                    &format!("localsend_file_{}", unknown_count)
                }
            };
            let mut file = File::create(file_path)?;
            copy(request.as_reader(), &mut file)?;

            // Response when complete
            request.respond(Response::empty(StatusCode::from(204)))?;

            // Reoutput table
            Terminal::clear_screen()?;
            Terminal::move_cursor(0, 0)?;
            file_status.insert(file_id.to_string(), "✅".to_string());
            draw_table(&prepare_upload, &file_status);
        }
    }

    Ok(())
}

fn announce(device: &Announce) -> Result<(), DynError> {
    let multicast_addr = Ipv4Addr::new(224, 0, 0, 167);
    let socket = UdpSocket::bind("0.0.0.0:0")?;
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

fn confirm_upload(prepare_upload: &PrepareUpload) -> Result<bool, DynError> {
    let file_status = HashMap::new();
    draw_table(prepare_upload, &file_status);

    Ok(Prompt::confirm("Receive?", true)?)
}

fn draw_table(prepare_upload: &PrepareUpload, file_staus: &HashMap<String, String>) {
    let files = &prepare_upload.files;
    let alias = &prepare_upload.info.alias;

    let mut table = Table::new(vec!["", "File Name", "Size", "File Type", "From", "Status"]);

    table
        .set_box_style(BoxStyle::Rounded)
        .set_column_alignment(5, Alignment::Center);

    for (num, (file_id, file_info)) in (1..).zip(files) {
        table.add_row(vec![
            &format!("{}", num),
            &file_info.file_name,
            &convert_storage_unit(file_info.size),
            &file_info.file_type,
            alias,
            file_staus.get(file_id).unwrap_or(&"".to_string()),
        ]);
    }

    table.print();
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
