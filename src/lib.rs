use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::thread::JoinHandle;

use colored::Colorize;
use exitfailure::ExitFailure;
use failure::ResultExt;
use structopt::StructOpt;
use threadpool::ThreadPool;

pub struct FileSearchResult {
    matched_lines: Vec<MatchResult>,
    file_name: Option<String>,
}

pub enum SearchInput {
    File(PathBuf),
    String(String),
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
    /// How many characters should be show as the context if the match
    #[structopt(short, long, default_value="0")]
    pub match_context: usize
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

        // important! if this channel is locked we have no debugging! since all print lines will try to acquire a lock on stdout
        let mut writer = std_out;//.lock();

        for search_result in receiver {
            if let Some(file_name) = search_result.file_name {
                if !search_result.matched_lines.is_empty() || options.include_empty_matches {
                    writeln!(writer, "\n{}", file_name.red()).unwrap();
                }
            }

            for r in search_result.matched_lines {
                //TODO: add options for only printing matches portion of the line with trailing and leading text
                if options.print_line_numbers {
                    writeln!(writer, "{:>4}. {}", (r.index + 1).to_string().blue(), r.result)
                } else {
                    writeln!(writer, "{}", r.result)
                }.with_context(|e| format!("Could not write to out: {}", e))
                    .unwrap();
            }
        }
    });

    (handle, sender)
}

pub fn create_search_processor(results_processor: Sender<FileSearchResult>) -> Result<(JoinHandle<()>, Sender<SearchMessage>), ExitFailure> {
    let (sender, receiver): (Sender<SearchMessage>, Receiver<SearchMessage>) = mpsc::channel();

    // receive the message in a dedicated thread
    let handle = thread::spawn(move || {

        // create thread pool for processor thread (don't know how to share it yet)
        let thread_pool = ThreadPool::new(8);

        // wait/block on receiver messages
        for search_message in receiver {
            // clone processor so we get the multi-producer implementation
            let results_processor = results_processor.clone();

            // kick off a worker for processing
            thread_pool.execute(move || {
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

                results_processor.send(FileSearchResult {
                    file_name,
                    matched_lines: search_reader(reader, search_message.options.match_context, &search_message.options.pattern),
                }).unwrap();
            });
        }

        // thread will exit as soon as corresponding channel sender gets drop (hanged up)
    });

    Ok((handle, sender))
}

fn search_reader(reader: impl BufRead, match_context: usize, pattern: &String) -> Vec<MatchResult> {
    let mut results = vec![];

    for (i, line) in reader.lines().enumerate() {
        let line = match line {
            Ok(line) => line,
            Err(_) => continue
        };

        // make context configurable....
        if let Some(result) = MatchResult::new(i, line).parse(match_context, pattern) {
            results.push(result);
        }
    }

    results
}

struct MatchResult {
    index: usize,
    raw: String,
    result: String
}

impl MatchResult {

    fn new(index: usize, raw: String) -> Self {
        MatchResult {
            index,
            raw,
            result: String::new()
        }
    }

    fn parse(mut self, context: usize, pattern: &String) -> Option<Self> {
        let p_len = pattern.len();
        let l_len = self.raw.len();

        let context = if context == 0 {
            l_len
        } else {
            context
        };

        match self.raw.find(pattern) {
            Some(m_start) => {
                let context_start = m_start as i32 - context as i32;
                let start: (usize, &str) = if context_start > 0 {
                    (context_start as usize, "...")
                } else {
                    (0, "")
                };

                let context_end = m_start + p_len + context;
                let end: (usize, &str) = if context_end >= l_len {
                    (l_len, "")
                } else {
                    (context_end, "...")
                };

                self.result += &*format!("{}{}{}{}{}",
                                      start.1,
                                      &self.raw[(start.0)..m_start],
                                      &self.raw[m_start..(m_start + p_len)].green(),
                                      &self.raw[m_start + p_len..(end.0)],
                                      end.1);

                Some(self)
            }
            _ => None
        }
    }
}

// ========================================================
// --------------------- Synchronous (single thread)
// ➜  grrs git:(master) ✗ cargo run -- "Piotr" ~/evb -ptm
// Completed in: 109.314656s

// --------------------- Asynchronous (Manually)
// Completed in: 91.771192s

// --------------------- My own Workers?
// 91.991514s probably made a mistake......

// --------------------- threadpool crate
// Completed in: 12.999917s
// yeah, i fucked something up

// ========================================================
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
//
// type Job = Box<dyn FnOnce() + Send + 'static>;
//
// pub enum WorkerMessage {
//     Start,
//     Job(Job),
//     Stop
// }
//
// pub struct ThreadPokkol {
//     _name: String,
//     sender: Mutex<Sender<WorkerMessage>>,
//     workers: Vec<JoinHandle<()>>
// }
//
// //TODO: create custom worker instead of raw handles in pool!!
// impl ThreadPool {
//
//     pub fn submit<F>(&mut self, job: F)
//     where
//         F: FnOnce(),
//         F: Send + 'static,
//     {
//         self.sender.lock().unwrap()
//             .send(WorkerMessage::Job(Box::new(job)))
//             .unwrap();
//     }
//
//     pub fn stop(self) {
//         for _i in 0..self.workers.len() {
//             self.sender.lock().unwrap().send(WorkerMessage::Stop).unwrap();
//         }
//
//         self.workers.into_iter()
//             .for_each(|handle| handle.join().unwrap());
//     }
//
//     pub fn new(name: String, num_workers: u8) -> ThreadPool {
//         let mut workers = vec![];
//
//         let (sender, receiver) = mpsc::channel();
//         let receiver = Arc::new(Mutex::new(receiver));
//
//         for i in 0..num_workers {
//             let worker_receiver = receiver.clone();
//             let worker_name = format!("{}_{}", &name, i);
//
//             workers.push(thread::spawn(move || {
//                 loop {
//                     let guard = worker_receiver.lock().unwrap();
//                     match guard.recv().unwrap() {
//                         WorkerMessage::Start => println!("started worker: {}", &worker_name),
//                         WorkerMessage::Job(job) => {
//                             let time = std::time::SystemTime::now();
//                             println!("running job for worker: {}", &worker_name);
//                             job();
//                             println!("completed job: {} in: {:?}", &worker_name, time.elapsed().unwrap());
//                         },
//                         WorkerMessage::Stop => {
//                             //println!("terminating worker: {}", &worker_name);
//                             break;
//                         }
//                     }
//                 }
//             }));
//         }
//
//         ThreadPool {
//             _name: name,
//             workers,
//             sender: Mutex::new(sender)
//         }
//     }
// }