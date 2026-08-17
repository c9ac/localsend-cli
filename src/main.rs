use localsend_cli::{DynError, receive, send};
use std::{path::PathBuf, process::exit};
use zfish::command::{App, Arg, Command};

fn main() -> Result<(), DynError> {
    smol::block_on(async {
        let app = App::new(env!("CARGO_PKG_NAME"))
            .version(format!("v{}", env!("CARGO_PKG_VERSION")))
            .about("A cli localsend client")
            .arg(
                Arg::new("alias")
                    .short('n')
                    .long("alias")
                    .default_value("my_device"),
            )
            .arg(
                Arg::new("port")
                    .short('p')
                    .long("port")
                    .default_value("53317"),
            )
            .subcommand(Command::new("receive").about("Prepare to receive file"))
            .subcommand(
                Command::new("send")
                    .about("Send file or text to other devices")
                    .arg(
                        Arg::new("file")
                            .short('f')
                            .long("file")
                            .takes_value(true)
                            .multiple(true),
                    )
                    .arg(Arg::new("timeout").short('t').long("timeout")),
            );

        let matches = app.get_matches();

        let alias = matches.value_of("alias").unwrap_or("my_device");
        let port = matches.value_of("port").unwrap_or("53317").parse()?;

        match matches.subcommand() {
            Some(("receive", _)) => receive(alias, port).await,
            Some(("send", sub_matches)) => {
                let files: Vec<PathBuf> = sub_matches
                    .values_of("file")
                    .ok_or("Please specify files")?
                    .iter()
                    .map(PathBuf::from)
                    .collect();

                let timeout = sub_matches.value_of("timeout").unwrap_or("5").parse()?;

                send(files, timeout, alias, port).await
            }
            _ => {
                eprintln!("Use --help for usage");
                exit(1);
            }
        }
    })
}
