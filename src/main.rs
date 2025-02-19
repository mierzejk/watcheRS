use std::{
    ffi::OsString,
    fmt, fmt::Formatter,
    fs::OpenOptions,
    io, io::Write,
    path, path::PathBuf,
    process,
    thread::sleep};
use clap::{arg, command, Parser, Subcommand};
use chrono::Local;
use expanduser::expanduser;

static FORMAT_NOW: &'static str = "%H:%M:%S";

#[derive(Subcommand)]
enum Action {
    /// [alias: r] Print out the full file contents, and then follow any incoming changes
    #[clap(alias = "r")]
    Read{
        /// second interval
        #[arg(short, long, default_value_t = 20u32)]
        sleep: u32,
    },
    /// [alias: w] Append current time to the file at specified intervals
    #[clap(alias = "w")]
    Write{
        /// millisecond interval
        #[arg(short, long, default_value_t = 2000u32)]
        interval: u32,
    }
}

fn action_fmt(action: &Action, f: &mut Formatter) -> fmt::Result {
    match action {
        Action::Read{ sleep: ref i} => write!(f, "Read with {:?} s sleep interval", *i),
        Action::Write{ interval: ref i} => write!(f, "Write at {:?} ms", *i)
    }
}

impl fmt::Debug for Action {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result { action_fmt(self, f) }
}

impl fmt::Display for Action {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result { action_fmt(self, f) }
}

/// Read, follow and print out any changes in the specified file; or append current time to the file at intervals
/// [default COMMAND: r(ead)]
#[derive(Parser)]
#[clap(version, verbatim_doc_comment)]
pub struct Cli {
    #[command(subcommand)]
    command: Option<Action>,
    /// Path to the file
    file: PathBuf
}

impl Cli {
    fn expand_path(&self) -> io::Result<PathBuf> {
        let file = self.file.clone().into_os_string().into_string().expect("Invalid path.");
        path::absolute(expanduser(file)?)
    }
}

#[inline(always)]
fn tail(file_path: PathBuf, sleep: &u32) {
    uu_tail::uumain([
        OsString::from("tail"),
        file_path.into_os_string(),
        OsString::from("--follow"),
        OsString::from(format!("--sleep-interval={}", *sleep))].into_iter());
}

fn write(file_path: PathBuf, interval: &u32) ->! {
    let duration = core::time::Duration::from_millis(*interval as u64);
    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(file_path).expect("Cannot open the file.");

    loop {
        let now = Local::now();
        let now_str = format!("{}.{:0>3}", now.format(FORMAT_NOW), now.timestamp_subsec_millis());
        writeln!(file, "{}", now_str).expect("Cannot append to the file.");
        file.flush().expect("Cannot flush the file.");
        println!("{}", now_str);
        file.sync_all().expect("Cannot synchronise the file.");
        sleep(duration);
    }
}

pub fn main() {
    let args = Cli::parse();
    let file = args.expand_path().unwrap();
    ctrlc::set_handler(move || {
        process::exit(0);
    }).expect("Cannot set SIGINT handler.");

    match args.command.unwrap_or(Action::Read{ sleep: 20u32 }) {
        Action::Read { sleep: ref interval } if file.is_file() => {
            println!("Following {:?}", file);
            tail(file, interval); }
        Action::Write { interval: ref sleep } => {
            println!("Writing to: {:?} every {} milliseconds.", file, *sleep);
            write(file, sleep); }
        _ => { println!("'{}' is not a file!", file.display()) }
    };

    process::exit(1)
}
