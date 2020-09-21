use colored::Colorize;
use exitfailure::ExitFailure;
use failure::ResultExt;
use std::io::{BufRead, Write};

/// Searches over the supplied input for the pattern supplied.
/// Writes output to the specified writer as so
pub fn find_matches<W: Write + ?Sized, R: BufRead>(print_line_numbers: bool,
                    pattern: &String,
                    writer: &mut Box<W>,
                    reader: R) -> Result<(), ExitFailure> {
    for (idx, line) in reader.lines().enumerate() {
        let line = line
            .with_context(|e| format!("Could not read line: {}", e))?;

        if line.contains(pattern) {
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

    use std::io::{BufReader, Write};
    use crate::find_matches;

    #[test]
    fn can_search_string() {
        let mut result: Box<Vec<u8>> = Box::new(Vec::new()); // vec u8 implements the write trait
        let text_to_search: String = "hello\nhow\nare you\ndoing".to_string();
        let pattern: String = "are ".to_string();

        // as_bytes is needed as &[u8] implements the BufRead trait
        let res = find_matches(false, &pattern.to_string(),
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

        find_matches(false, &String::new(), &mut result, source.as_bytes())
            .expect("should find results");

        let result = String::from_utf8_lossy(&result);

        assert_eq!(result.trim(), source);
    }
}
