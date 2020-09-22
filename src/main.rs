use std::fmt::Debug;
use std::fs::File;
use std::io::{self, BufReader, Write};
use std::path::{Path, PathBuf};

use colored::Colorize;
use exitfailure::ExitFailure;
use failure::ResultExt;
use structopt::StructOpt;

fn main() -> Result<(), ExitFailure> {
    let time = std::time::SystemTime::now();
    let args = Cli::from_args();

    let std_out = io::stdout();
    // allocate the pointer to stdout on the heap
    let mut out = Box::new(std_out.lock());
    let path: &Option<PathBuf> = &args.path;

    let res = match path {
        Some(path) => {
            search_path(path, &mut out, &args)
        }
        _ => {
            let input = io::stdin();
            let path: Option<&Path> = None;
            grrs::find_matches(args.print_line_numbers, path, &args.pattern, &mut out, input.lock())
        }
    };

    if args.time {
        println!("Completed in: {:?}", time.elapsed().unwrap());
    }

    res
}

fn search_path(path: impl AsRef<std::path::Path> + Debug,
               mut out: &mut Box<impl Write + ?Sized>, args: &Cli) -> Result<(), ExitFailure> {
    let path_ref = path.as_ref();
    if path_ref.is_dir() {
        let dir_iter = match path_ref.read_dir() {
            Ok(dir) => dir,
            // we do not have permissions to open some directories
            Err(_) => return Ok(())
        };

        for entry in dir_iter {
            let entry = entry?;
            let entry_type = entry.file_type()?;

            if entry_type.is_file() {
                search_file(&entry.path(), &mut out, args)?
            } else if entry_type.is_dir() {
                search_path(&entry.path(), &mut out, args)?
            }
        }
    }

    Ok(())
}

fn search_file<P: AsRef<std::path::Path> + Debug>(path: &P,
                                                  mut out: &mut Box<impl Write + ?Sized>,
                                                  args: &Cli) -> Result<(), ExitFailure> {
    let name_for_matches = if !args.matched {
        grrs::print_path(path);
        None
    } else{
        Some(path)
    };

    let file = File::open(path)
        .with_context(|_e| format!("{} {:?}", "Error reading".red(), path))?;

    let reader = BufReader::new(file);
    grrs::find_matches(args.print_line_numbers, name_for_matches, &args.pattern, &mut out, reader)
}

/// Searches for the pattern in the supplied file using Rust!
#[derive(StructOpt)]
struct Cli {
    /// The pattern to look for
    pattern: String,
    /// The path to the file to read
    #[structopt(parse(from_os_str))]
    path: Option<std::path::PathBuf>,
    /// Should line numbers be printed
    #[structopt(short, long)]
    print_line_numbers: bool,
    /// Should the search time be shows
    #[structopt(short, long)]
    time: bool,
    /// Should only matches be shown
    #[structopt(short, long)]
    matched: bool,
}
