use crate::{DynError, announce, convert_storage_unit, protocol::*};
use miniserde::json;
use std::{collections::HashMap, fs::File, io::Write};
use tiny_http::{Method, Response, Server, StatusCode};
use zfish::{
    Alignment, ProgressBar, ProgressStyle, Prompt, Terminal,
    table::{BoxStyle, Table},
};

pub async fn receive(alias: &str, port: usize) -> Result<(), DynError> {
    let device = Announce::new(alias, port);

    announce(&device).await?;

    let server = Server::http(format!("0.0.0.0:{}", port))?;
    let mut prepare_upload = PrepareUpload::default();
    let mut unknown_count = 0;
    let mut file_status = HashMap::new();

    for mut request in server.incoming_requests() {
        // Listen to new urls if prepare upload request occur
        if request.url() == PREPARE_UPLOAD_URI && request.method() == &Method::Post {
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
        else if request.url().starts_with(UPLOAD_PREFFIX) && request.method() == &Method::Post {
            let (mut file, file_size, file_id) =
                receive_file(request.url(), &prepare_upload, &mut unknown_count)?;

            // Setup progress bar
            let mut progress_bar = ProgressBar::new(file_size).with_style(ProgressStyle::Arrow);

            // Show `downloading` status
            file_status.insert(file_id.to_string(), "downloading".to_string());
            draw_table(&prepare_upload, &file_status)?;

            // Write to file
            let mut buffer = [0; 8192];
            let mut downloaded = 0;
            loop {
                let n = request.as_reader().read(&mut buffer)?;
                if n == 0 {
                    break;
                }
                file.write_all(&buffer[..n])?;

                // Update progress bar
                downloaded += n;
                progress_bar.set(downloaded as u64);
            }

            // Response when complete
            request.respond(Response::empty(StatusCode::from(204)))?;

            // Reoutput table
            file_status.insert(file_id.to_string(), "✅".to_string());
            draw_table(&prepare_upload, &file_status)?;
        }
    }

    Ok(())
}

fn confirm_upload(prepare_upload: &PrepareUpload) -> Result<bool, DynError> {
    let file_status = HashMap::new();
    draw_table(prepare_upload, &file_status)?;

    Ok(Prompt::confirm("Receive?", true)?)
}

fn draw_table(
    prepare_upload: &PrepareUpload,
    file_staus: &HashMap<String, String>,
) -> Result<(), DynError> {
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

    Terminal::clear_screen()?;
    Terminal::move_cursor(0, 0)?;
    table.print();

    Ok(())
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

fn receive_file(
    url: &str,
    prepare_upload: &PrepareUpload,
    unknown_count: &mut usize,
) -> Result<(File, u64, String), DynError> {
    let file_id = url
        .split("?")
        .nth(1)
        .ok_or("500")?
        .split("&")
        .find(|s| s.starts_with("fileId="))
        .and_then(|s| s.get(7..))
        .map(|s| s.to_string())
        .ok_or("500")?;

    let file_path = match prepare_upload.files.get(&file_id) {
        Some(file_info) => &file_info.file_name,
        None => {
            *unknown_count += 1;
            &format!("localsend_file_{}", unknown_count)
        }
    };

    let file_size = prepare_upload.files.get(&file_id).ok_or("500")?.size;

    let file = File::create(file_path)?;
    Ok((file, file_size, file_id))
}
