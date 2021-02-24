use app_dirs::data_root;
use chrono::{DateTime, Utc};
use log::{debug, info, warn};

use std::io::{self, BufRead, BufReader};
use std::{
    fmt::{self, Display},
    fs::File,
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub struct Task {
    pub schedule: String,
    pub program: String,
    pub next_iter: DateTime<Utc>,
}

impl Display for Task {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.schedule, self.program)
    }
}

pub fn read_crontab<P>(path: P) -> io::Result<Vec<Task>>
where
    P: AsRef<Path>,
{
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
                for p in sched_parts {
                    s.push_str(&format!("{} ", p));
                }

                let program_parts = parts[5..].to_vec();
                let mut p = String::new();
                for part in program_parts {
                    p.push_str(&format!("{} ", part));
                }
                if let Ok(next_iter) = cron_parser::parse(&s, &Utc::now()) {
                    let t = Task {
                        schedule: s.trim().to_string(),
                        program: p.trim().to_string(),
                        next_iter,
                    };
                    debug!("built task: {:?}", t);
                    tasks.push(t);
                }
            }
            Err(e) => warn!("Ignoring line {}: {}", line_number, e),
        }
        line_number += 1;
    }

    Ok(tasks)
}

pub fn get_earliest_task(tasks: &Vec<Task>) -> DateTime<Utc> {
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

pub fn crontab_path() -> PathBuf {
    let mut path = data_root(app_dirs::AppDataType::UserConfig).unwrap();
    path.push("crontab");
    path
}
