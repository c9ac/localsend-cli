use zfish::command::{App, Arg, Command};

fn main() {
    let app = App::new(env!("CARGO_PKG_NAME"))
        .version(format!("v{}", env!("CARGO_PKG_VERSION")))
        .about("A cli localsend client")
        .arg(Arg::new("name").short('n').long("name"))
        .subcommand(Command::new("receive").about("Prepare to receive file"))
        .subcommand(
            Command::new("send")
                .about("Send file or text to other devices")
                .arg(Arg::new("file").short('f').long("file").required(true)),
        );

    let matches = app.get_matches();

    match matches.subcommand() {
        Some(("receive", _)) => todo!(),
        Some(("send", sub_matches)) => {
            let file = sub_matches.value_of("file");
            todo!();
        }
        _ => eprintln!("Use --help for usage"),
    }
}
