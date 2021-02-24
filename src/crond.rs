use chrono::Utc;
use clap::{App, Arg};
use cron_parser::parse;
use log::{debug, warn};
use std::{process, thread};

fn main() {
    pretty_env_logger::init_custom_env("CRON_LOG");

    let matches = App::new(env!("CARGO_PKG_NAME"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about("A Rust implementation of a cron daemon.")
        .version(env!("CARGO_PKG_VERSION"))
        .arg(
            Arg::with_name("crontab")
                .long("crontab")
                .short("c")
                .help("Override the location of the crontab file.")
                .takes_value(true)
                .value_name("CRONTAB"),
        )
        .get_matches();

    // Read config file
    let mut tasks = crond::read_crontab(
        matches
            .value_of("crontab")
            .unwrap_or(crond::crontab_path().to_str().unwrap()),
    )
    .expect("failed to read crontab");
    if tasks.len() == 0 {
        warn!("No tasks! Quitting...");
        process::exit(0);
    }

    let mut next_due_task = crond::get_earliest_task(&tasks);
    loop {
        // Wait until next due task
        let time_now = Utc::now();
        let wait_for = next_due_task - time_now;
        debug!("waiting {}s for next task...", wait_for.num_seconds());
        if let Ok(dur) = wait_for.to_std() {
            thread::sleep(dur);
        }

        // Run tasks that are due
        let time_now = Utc::now();
        for task in &mut tasks {
            if task.next_iter < time_now {
                task.next_iter = parse(&task.schedule, &time_now).unwrap();
                let args = task.program.clone();
                debug!("Running {:?}", args);
                thread::spawn(move || {
                    #[cfg(windows)]
                    let res = process::Command::new("cmd").arg("/c").arg(&args).spawn();
                    #[cfg(not(windows))]
                    let res = process::Command::new("sh").arg("-c").arg(&args).spawn();
                    match res {
                        Ok(_) => (),
                        Err(e) => warn!("Failed to run task `{}`: {}", args, e),
                    }
                });
            }
        }

        // Find next due time
        next_due_task = crond::get_earliest_task(&tasks);
    }
}
