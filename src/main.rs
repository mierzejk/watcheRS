use std::{
    ffi::OsString,
    fmt, fmt::Formatter,
    fs::{File, OpenOptions},
    io, io::Write,
    ops::DerefMut,
    os::unix::fs::MetadataExt,
    path, path::PathBuf,
    process,
    thread::sleep};
use clap::{arg, command, Parser, Subcommand};
use chrono::Local;
use expanduser::expanduser;
use file_guard::Lock;

static FORMAT_NOW: &'static str = "%H:%M:%S";
static LINE_SIZE: usize = 14usize;

#[derive(Subcommand)]
enum Action {
    /// [alias: r] Print out the last line in the file, and then follow any incoming changes
    #[clap(alias = "r")]
    Read{
        /// second interval
        #[arg(short, long, default_value_t = 10u32)]
        sleep: u32,
        #[clap(long, short='p', required = false, default_value_t = false)]
        /// Disable inotify and employ polling instead
        use_polling: bool
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
        Action::Read{ sleep: ref i, use_polling: ref polling} =>
            write!(f, "Read with {:?} s sleep interval with {}", *i, if *polling { "polling" } else { "inotify subsystem" }),
        Action::Write{ interval: ref i} =>
            write!(f, "Write at {:?} ms", *i)
    }
}

impl fmt::Debug for Action {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result { action_fmt(self, f) }
}

impl fmt::Display for Action {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result { action_fmt(self, f) }
}

/// Read the last line, follow and print out any changes in the specified file; or append current time to the file at intervals
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
fn tail(file_path: PathBuf, sleep: &u32, use_polling: &bool) {
    let mut args = vec![
        OsString::from("tail"),
        file_path.into_os_string(),
        OsString::from("--lines=1"),
        OsString::from("--follow=descriptor"),
        OsString::from(format!("--sleep-interval={}", *sleep))];
    if *use_polling {
        args.push(OsString::from("--use-polling"));
    }

    uu_tail::uumain(args.into_iter());
}

fn write_line(mut file: &File) -> io::Result<()> {
    let now = Local::now();
    let now_str = format!("{}.{:0>3}", now.format(FORMAT_NOW), now.timestamp_subsec_millis());
    writeln!(file, "{}", now_str)?;
    file.flush()?;
    println!("{}", now_str);
    file.sync_data()?;
    Ok(())
}

fn get_size(file: &File) -> io::Result<usize> {
    file.sync_all()?;
    usize::try_from(file.metadata()?.size()).or_else(
        |err| Err(io::Error::new(io::ErrorKind::InvalidData, err.to_string()))
    )
}

fn write(file_path: PathBuf, interval: &u32) ->! {
    let duration = core::time::Duration::from_millis(*interval as u64);
    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(file_path).expect("Cannot open the file.");

    loop {
        sleep(duration);
        let file_size = get_size(&file).expect("Cannot get the file size.");
        let lock_result = file_guard::try_lock(
            &mut file,
            Lock::Exclusive,
            usize::MIN,
            file_size + LINE_SIZE);
        let Ok(mut lock) = lock_result else {
            println!("WARN: Cannot lock the file; append skipped.");
            continue;
        };
        write_line(lock.deref_mut()).expect("Cannot append a line to the file.");
        drop(lock);
    }
}

pub fn main() {
    let args = Cli::parse();
    let file = args.expand_path().unwrap();
    ctrlc::set_handler(move || {
        process::exit(0);
    }).expect("Cannot set SIGINT handler.");

    match args.command.unwrap_or(Action::Read{ sleep: 10u32, use_polling: false }) {
        Action::Read { sleep: ref interval, use_polling: ref polling } if file.is_file() => {
            println!("Following {:?} file descriptor using {}", file, if *polling { "polling." } else { "inotify subsystem." });
            tail(file, interval, polling); }
        Action::Write { interval: ref sleep } => {
            println!("Writing to: {:?} every {} milliseconds.", file, *sleep);
            write(file, sleep); }
        _ => { println!("'{}' is not a file!", file.display()) }
    };

    process::exit(1)
}
