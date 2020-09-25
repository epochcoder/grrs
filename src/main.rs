
use std::fmt::Debug;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::Sender;

use exitfailure::ExitFailure;
use structopt::StructOpt;

use grrs::{SearchInput, SearchMessage, SearchOptions};

fn main() -> Result<(), ExitFailure> {
    let time = std::time::SystemTime::now();
    let args: Arc<SearchOptions> = Arc::new(grrs::SearchOptions::from_args());
    let path: &Option<PathBuf> = &args.path;

    let (h1, results_proc) = grrs::create_results_processor(args.clone());
    let (h2, search_processor) = grrs::create_search_processor(results_proc)?;

    let res = match path {
        Some(path) => {
            search_path(path, search_processor, args.clone())
        }
        _ => {
            drop(search_processor);
            Ok(())
        }
    };

    h1.join().unwrap();
    h2.join().unwrap();

    if args.time {
        println!("Completed in: {:?}", time.elapsed().unwrap());
    }

    res
}

fn search_path(path: impl AsRef<std::path::Path> + Debug,
               search_processor: Sender<SearchMessage>,
               options: Arc<SearchOptions>) -> Result<(), ExitFailure> {
    let path_ref = path.as_ref();
    if path_ref.is_dir() {
        let dir_iter = match path_ref.read_dir() {
            Ok(dir) => dir,
            Err(_) => return Ok(())
        };

        for entry in dir_iter {
            search_path(entry?.path(), search_processor.clone(), options.clone())?
        }
    } else if path_ref.is_file() {
        search_processor.send(SearchMessage::new(
            SearchInput::File(path_ref.to_path_buf()), options.clone()))?;
    }

    Ok(())
}