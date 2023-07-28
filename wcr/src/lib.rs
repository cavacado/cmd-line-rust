use clap::{App, Arg};
use std::error::Error;
use std::fs::File;
use std::io::{self, BufRead, BufReader};

type MyResult<T> = Result<T, Box<dyn Error>>;

#[derive(Debug)]
pub struct Config {
    files: Vec<String>,
    lines: bool,
    words: bool,
    bytes: bool,
    chars: bool,
}

#[derive(Debug, PartialEq)]
pub struct FileInfo {
    num_lines: usize,
    num_words: usize,
    num_bytes: usize,
    num_chars: usize,
}

pub fn get_args() -> MyResult<Config> {
    let matches = App::new("wcr")
        .version("0.1.0")
        .author("cavacado <zl@zl.com>")
        .about("rust wc")
        .arg(
            Arg::with_name("files")
                .value_name("FILE")
                .help("Input file(s)")
                .default_value("-")
                .multiple(true),
        )
        .arg(
            Arg::with_name("lines")
                .short("l")
                .long("lines")
                .help("Show line count")
                .takes_value(false),
        )
        .arg(
            Arg::with_name("words")
                .short("w")
                .long("words")
                .help("Show word count")
                .takes_value(false),
        )
        .arg(
            Arg::with_name("bytes")
                .short("c")
                .long("bytes")
                .help("Show byte count")
                .takes_value(false),
        )
        .arg(
            Arg::with_name("chars")
                .short("m")
                .long("chars")
                .help("Show character count")
                .takes_value(false)
                .conflicts_with("bytes"),
        )
        .get_matches();

    let mut lines = matches.is_present("lines");
    let mut words = matches.is_present("words");
    let mut bytes = matches.is_present("bytes");
    let chars = matches.is_present("chars");

    if [lines, words, bytes, chars].iter().all(|v| v == &false) {
        lines = true;
        words = true;
        bytes = true;
    }

    Ok(Config {
        files: matches.values_of_lossy("files").unwrap(),
        lines,
        words,
        bytes,
        chars: matches.is_present("chars"),
    })
}

fn format_field(value: usize, show: bool) -> String {
    if show {
        format!("{:>8}", value)
    } else {
        format!("")
    }
}

fn print_cols(config: &Config, info: &FileInfo, filename: &String) {
    let lines_str = format_field(info.num_lines, config.lines);
    let words_str = format_field(info.num_words, config.words);
    let bytes_str = format_field(info.num_bytes, config.bytes);
    let chars_str = format_field(info.num_chars, config.chars);
    let filename_str = if filename == "-" {
        format!("")
    } else {
        format!(" {filename}")
    };

    println!(
        "{}{}{}{}",
        lines_str,
        words_str,
        if config.bytes { bytes_str } else { chars_str },
        filename_str
    );
}

pub fn run(config: Config) -> MyResult<()> {
    let num_files = config.files.len();
    let mut infos = Vec::new();
    for filename in config.files.iter() {
        match open(filename) {
            Err(err) => eprintln!("{}: {}", filename, err),
            Ok(file) => {
                let info = count(file)?;
                print_cols(&config, &info, filename);
                infos.push(info);
            }
        }
    }
    if num_files > 1 {
        let total = infos.iter().fold(
            FileInfo {
                num_lines: 0,
                num_bytes: 0,
                num_chars: 0,
                num_words: 0,
            },
            |acc, info| FileInfo {
                num_lines: acc.num_lines + info.num_lines,
                num_bytes: acc.num_bytes + info.num_bytes,
                num_chars: acc.num_chars + info.num_chars,
                num_words: acc.num_words + info.num_words,
            },
        );
        let name = String::from("total");
        print_cols(&config, &total, &name)
    }
    Ok(())
}

fn open(filename: &str) -> MyResult<Box<dyn BufRead>> {
    match filename {
        "-" => Ok(Box::new(BufReader::new(io::stdin()))),
        _ => Ok(Box::new(BufReader::new(File::open(filename)?))),
    }
}

pub fn count(mut file: impl BufRead) -> MyResult<FileInfo> {
    let mut num_lines = 0;
    let mut num_words = 0;
    let mut num_bytes = 0;
    let mut num_chars = 0;

    let mut line = String::new();
    loop {
        let bytes = file.read_line(&mut line)?;
        if bytes == 0 {
            break;
        }
        num_lines += 1;
        num_words += line.split_whitespace().count();
        num_chars += line.chars().count();
        num_bytes += line.bytes().count();
        line.clear();
    }

    Ok(FileInfo {
        num_lines,
        num_words,
        num_bytes,
        num_chars,
    })
}

#[cfg(test)]
mod tests {
    use super::{count, FileInfo};
    use std::io::Cursor;

    #[test]
    fn test_count() {
        let text = "I don't want the world. I just want your half.\r\n";
        let info = count(Cursor::new(text));
        assert!(info.is_ok());
        let expected = FileInfo {
            num_lines: 1,
            num_words: 10,
            num_chars: 48,
            num_bytes: 48,
        };
        assert_eq!(info.unwrap(), expected);
    }
}
