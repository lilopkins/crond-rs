use std::{fs::File, path::Path, process, thread};
use std::io::{self, BufRead, BufReader};
use chrono::{DateTime, Utc};
use clap::{App, Arg};
use cron_parser::parse;
use log::{debug, info, warn};

#[derive(Debug)]
struct Task {
    schedule: String,
    program: String,
    next_iter: DateTime<Utc>,
}

fn main() {
    pretty_env_logger::init();

    let matches = App::new(env!("CARGO_PKG_NAME"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about("A Rust implementation of a cron server.")
        .version(env!("CARGO_PKG_VERSION"))
        .arg(Arg::with_name("crontab")
                .long("crontab")
                .short("c")
                .help("Override the location of the crontab file.")
                .takes_value(true)
                .value_name("CRONTAB")
                .default_value(if cfg!(windows) { r"C:\crontab" } else { r"/etc/crontab" })
        )
        .get_matches();

    // Read config file
    let mut tasks = read_crontab(matches.value_of("crontab").unwrap()).expect("failed to read crontab");
    if tasks.len() == 0 {
        warn!("No tasks! Quitting...");
        process::exit(0);
    }

    let mut next_due_task = get_earliest_task(&tasks);
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
                    let res = process::Command::new("cmd")
                        .arg("/c")
                        .arg(&args)
                        .spawn();
                    #[cfg(not(windows))]
                    let res = process::Command::new("sh")
                        .arg("-c")
                        .arg(&args)
                        .spawn();
                    match res {
                        Ok(_) => (),
                        Err(e) => warn!("Failed to run task `{}`: {}", args, e),
                    }
                });
            }
        }

        // Find next due time
        next_due_task = get_earliest_task(&tasks);
    }
}

fn read_crontab<P>(path: P) -> io::Result<Vec<Task>> where P: AsRef<Path> {
    info!("Reading crontab from {:?}...", path.as_ref());
    let file = BufReader::new(File::open(path)?);
    let mut tasks = Vec::new();
    
    let mut line_number = 1;
    for line in file.lines() {
        debug!("crontab line {}: {:?}", line_number, line);
        match line {
            Ok(line) => {
                if line.trim().starts_with('#') {
                    // Comment
                    line_number += 1;
                    continue;
                }

                // Attempt to parse.
                let parts = line.split_whitespace().collect::<Vec<&str>>();
                
                let sched_parts = parts[0..5].to_vec();
                let mut s = String::new();
                for p in sched_parts { s.push_str(&format!("{} ", p)); }

                let program_parts = parts[5..].to_vec();
                let mut p = String::new();
                for part in program_parts { p.push_str(&format!("{} ", part)); }
                if let Ok(next_iter) = cron_parser::parse(&s, &Utc::now()) {
                    let t = Task {
                        schedule: s,
                        program: p,
                        next_iter,
                    };
                    debug!("built task: {:?}", t);
                    tasks.push(t);
                }
            },
            Err(e) => warn!("Ignoring line {}: {}", line_number, e),
        }
        line_number += 1;
    }

    Ok(tasks)
}

fn get_earliest_task(tasks: &Vec<Task>) -> DateTime<Utc> {
    let mut earliest = None;

    for t in tasks {
        if let Some(e) = earliest {
            if t.next_iter < e {
                earliest = Some(t.next_iter.clone());
            }
        } else {
            earliest = Some(t.next_iter.clone());
        }
    }

    earliest.unwrap()
}
