use app_dirs::data_root;
use clap::{App, Arg};
use process::Command;
use rustyline::Editor;

use std::{
    env,
    fs::{self, File},
};
use std::{io, process};

fn main() -> io::Result<()> {
    pretty_env_logger::init_custom_env("CRON_LOG");

    let matches = App::new("crontab")
        .author(env!("CARGO_PKG_AUTHORS"))
        .about("A tool to manage cron tables.")
        .version(env!("CARGO_PKG_VERSION"))
        .arg(
            Arg::with_name("find")
                .short("f")
                .required_unless_one(&["edit", "list", "remove"])
                .conflicts_with_all(&["edit", "list", "remove"])
                .help("find user's crontab"),
        )
        .arg(
            Arg::with_name("edit")
                .short("e")
                .required_unless_one(&["find", "list", "remove"])
                .conflicts_with_all(&["find", "list", "remove"])
                .help("edit user's crontab"),
        )
        .arg(
            Arg::with_name("list")
                .short("l")
                .required_unless_one(&["find", "edit", "remove"])
                .conflicts_with_all(&["find", "edit", "remove"])
                .help("list user's crontab"),
        )
        .arg(
            Arg::with_name("remove")
                .short("r")
                .required_unless_one(&["find", "edit", "list"])
                .conflicts_with_all(&["find", "edit", "list"])
                .help("delete user's crontab"),
        )
        .arg(
            Arg::with_name("interactive")
                .short("i")
                .help("prompt before deleting"),
        )
        .arg(
            Arg::with_name("crontab")
                .short("c")
                .help("override the location of the crontab")
                .takes_value(true)
                .value_name("crontab"),
        )
        .get_matches();

    let crontab_path = crond::crontab_path();
    let crontab_path = matches
        .value_of("crontab")
        .unwrap_or(crontab_path.to_str().unwrap());

    if matches.is_present("find") {
        println!("{}", crontab_path);
    } else if matches.is_present("list") {
        if fs::metadata(crontab_path).is_err() {
            eprintln!("no crontab for this user");
            process::exit(1);
        }
        if let Ok(tasks) = crond::read_crontab(crontab_path) {
            for t in tasks {
                println!("{}", t);
            }
        }
    } else if matches.is_present("edit") {
        let mut temp_crontab_path = data_root(app_dirs::AppDataType::UserCache).unwrap();
        temp_crontab_path.push(".crontab.edit");
        if fs::metadata(crontab_path).is_err() {
            eprintln!("no crontab for user - using an empty one");
            File::create(&temp_crontab_path).unwrap();
        } else {
            // Make temp copy of file.
            fs::copy(crontab_path, &temp_crontab_path).unwrap();
        }
        // Open editor
        #[cfg(windows)]
        let editor = env::var("EDITOR").unwrap_or(r"C:\Windows\notepad.exe".to_string());
        #[cfg(not(windows))]
        let editor = env::var("EDITOR").unwrap_or(r"/usr/bin/vi".to_string());
        loop {
            Command::new(&editor)
                .arg(temp_crontab_path.to_str().unwrap())
                .status()
                .unwrap();

            // Verify file and install.
            // TODO: Detect for changes.
            eprintln!("crontab: installing new crontab");
            if crond::read_crontab(&temp_crontab_path).is_ok() {
                fs::copy(temp_crontab_path, crontab_path).unwrap();
                break;
            } else {
                eprintln!("errors in crontab file, can't install!");
                let mut rl = Editor::<()>::new();
                match rl.readline("Do you want to retry the same edit? (Y/N) ") {
                    Ok(line) => {
                        if line.to_lowercase() == "n" {
                            break;
                        }
                    }
                    _ => break,
                }
            }
        }
    } else if matches.is_present("remove") {
        if fs::metadata(crontab_path).is_err() {
            eprintln!("no crontab for this user");
            process::exit(1);
        }
        if matches.is_present("interactive") {
            let mut rl = Editor::<()>::new();
            match rl.readline("crontab: really delete users crontab? ") {
                Ok(line) => {
                    if line.to_lowercase() != "y" {
                        process::exit(0);
                    }
                }
                _ => process::exit(0),
            }
        }
        fs::remove_file(crontab_path).unwrap();
    }

    Ok(())
}
