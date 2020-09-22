use std::io::{BufRead, Write};
use std::path::Path;

use colored::Colorize;
use exitfailure::ExitFailure;
use failure::ResultExt;

/// prints a path based on a toggle provided
/// returns true if the path was printed
pub fn print_path<P: AsRef<Path> + ?Sized>(path: &P) {
    let pretty_name = path.as_ref()
        .file_name().unwrap()
        .to_string_lossy().red();

    println!("\n{}", pretty_name);
}

/// Searches over the supplied input for the pattern supplied.
/// Writes output to the specified writer as so
pub fn find_matches(print_line_numbers: bool, file_name_to_print: Option<impl AsRef<Path>>, pattern: &String, writer: &mut Box<impl Write + ?Sized>, reader: impl BufRead) -> Result<(), ExitFailure> {
    let mut printed = false;
    for (idx, line) in reader.lines().enumerate() {
        let line = match line {
            Ok(line) => line,
            // line might not have valid UTF-8 so we dont want to quit here
            Err(_) => continue
        };

        if line.contains(pattern) {
            if !printed {
                if let Some(ref path) = file_name_to_print {
                    print_path(path);
                    printed = true;
                }
            }

            if print_line_numbers {
                writeln!(writer, "{:>4}. {}", (idx + 1).to_string().blue(), line)
            } else {
                writeln!(writer, "{}", line)
            }
                .with_context(|e| format!("Could not write to out: {}", e))?;
        }
    }

    Ok(())
}

// ensure this is only compiled for the test module
#[cfg(test)]
mod tests {
    use std::io::BufReader;

    use crate::find_matches;

    #[test]
    fn can_search_string() {
        let mut result: Box<Vec<u8>> = Box::new(Vec::new()); // vec u8 implements the write trait
        let text_to_search: String = "hello\nhow\nare you\ndoing".to_string();
        let pattern: String = "are ".to_string();

        // as_bytes is needed as &[u8] implements the BufRead trait
        let res = find_matches(false, None, &pattern.to_string(),
                               &mut result, BufReader::new(text_to_search.as_bytes()));
        assert!(res.is_ok());

        let result = String::from_utf8_lossy(&result);
        let expected = String::from("are you");

        assert_eq!(result.trim(), expected)
    }

    #[test]
    fn shows_all_lines_when_empty() {
        let mut result: Box<Vec<u8>> = Box::new(Vec::new());
        let source = "Should\nshow all\nlines";

        find_matches(false, None, &String::new(), &mut result, source.as_bytes())
            .expect("should find results");

        let result = String::from_utf8_lossy(&result);

        assert_eq!(result.trim(), source);
    }
}
