use std::fs::File;
use std::io::{self, BufRead, BufReader, stdin, Write};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::thread::JoinHandle;

use colored::Colorize;
use exitfailure::ExitFailure;
use failure::ResultExt;
use structopt::StructOpt;

pub struct FileSearchResult {
    matched_lines: Vec<(usize, String)>,
    file_name: Option<String>,
}

pub enum SearchInput {
    File(PathBuf),
    String(String)
}

pub struct SearchMessage {
    options: Arc<SearchOptions>,
    input: SearchInput,
}

/// Searches for the pattern in the supplied file using Rust!
#[derive(StructOpt)]
pub struct SearchOptions {
    /// The pattern to look for
    pub pattern: String,
    /// The path to the file to read
    #[structopt(parse(from_os_str))]
    pub path: Option<std::path::PathBuf>,
    /// Should line numbers be printed
    #[structopt(short, long)]
    pub print_line_numbers: bool,
    /// Should the search time be shows
    #[structopt(short, long)]
    pub time: bool,
    /// Include empty matches
    #[structopt(short, long)]
    pub include_empty_matches: bool,
}

impl SearchMessage {
    pub fn new(input: SearchInput, options: Arc<SearchOptions>) -> SearchMessage {
        SearchMessage {
            options,
            input,
        }
    }
}

pub fn create_results_processor(options: Arc<SearchOptions>) -> (JoinHandle<()>, Sender<FileSearchResult>) {
    let (sender, receiver): (Sender<FileSearchResult>, Receiver<FileSearchResult>) = mpsc::channel();

    let handle = thread::spawn(move || {
        let std_out = io::stdout();
        let mut writer = std_out.lock();

        for search_result in receiver {
            if let Some(file_name) = search_result.file_name {
                if !search_result.matched_lines.is_empty() || options.include_empty_matches {
                    writeln!(writer, "\n{}", file_name.red()).unwrap();
                }
            }

            for (index, line) in search_result.matched_lines {
                if options.print_line_numbers {
                    writeln!(writer, "{:>4}. {}", (index + 1).to_string().blue(), line)
                } else {
                    writeln!(writer, "{}", line)
                }.with_context(|e| format!("Could not write to out: {}", e))
                    .unwrap();
            }
        }
    });

    (handle, sender)
}

pub fn create_search_processor(results_processor: Sender<FileSearchResult>) -> Result<(JoinHandle<()>, Sender<SearchMessage>), ExitFailure> {
    let (sender, receiver): (Sender<SearchMessage>, Receiver<SearchMessage>) = mpsc::channel();

    let handle = thread::spawn(move || {
        for search_message in receiver {
            let (reader, file_name) = match &search_message.input {
                SearchInput::File(path) => {
                    let file = File::open(path)
                        .with_context(|_e| format!("{} {:?}", "Error reading".red(), path))
                        .unwrap();

                    (Box::new(BufReader::new(file)) as Box<dyn BufRead>, Option::Some(path.display().to_string()))
                }
                SearchInput::String(string) => {
                    (Box::new(BufReader::new(string.as_bytes())) as Box<dyn BufRead>, Option::None)
                }
            };

            let results = search_reader(reader, &search_message.options.pattern);

            results_processor.send(FileSearchResult {
                file_name,
                matched_lines: results,
            }).unwrap()
        }
    });

    Ok((handle, sender))
}

fn search_reader(reader: impl BufRead, pattern: &String) -> Vec<(usize, String)> {
    let mut results = vec![];

    for (i, line) in reader.lines().enumerate() {
        let line = match line {
            Ok(line) => line,
            Err(_) => continue
        };

        if line.contains(pattern) {
            results.push((i, line));
        }
    }

    results
}

// Synchronous
// ➜  grrs git:(master) ✗ cargo run -- "Piotr" ~/evb -ptm
// Completed in: 109.314656s

// Asynchronous (Manually)
// Completed in: 91.771192s

// Workers?

// ensure this is only compiled for the test module

// #[cfg(test)]
// mod tests {
//     use std::io::BufReader;
//
//     use crate::find_matches;
//
//     #[test]
//     fn can_search_string() {
//         let mut result: Box<Vec<u8>> = Box::new(Vec::new()); // vec u8 implements the write trait
//         let text_to_search: String = "hello\nhow\nare you\ndoing".to_string();
//         let pattern: String = "are ".to_string();
//
//         // as_bytes is needed as &[u8] implements the BufRead trait
//         let res = find_matches(false, None, &pattern.to_string(),
//                                &mut result, BufReader::new(text_to_search.as_bytes()));
//         assert!(res.is_ok());
//
//         let result = String::from_utf8_lossy(&result);
//         let expected = String::from("are you");
//
//         assert_eq!(result.trim(), expected)
//     }
//
//     #[test]
//     fn shows_all_lines_when_empty() {
//         let mut result: Box<Vec<u8>> = Box::new(Vec::new());
//         let source = "Should\nshow all\nlines";
//
//         find_matches(false, None, &String::new(), &mut result, source.as_bytes())
//             .expect("should find results");
//
//         let result = String::from_utf8_lossy(&result);
//
//         assert_eq!(result.trim(), source);
//     }
// }
