use clap::{App, Arg};
use std::error::Error;
use std::fs::File;
use std::io::{self, BufRead, BufReader};

#[derive(Debug)]
pub struct Config {
    pub files: Vec<String>,
    pub number_lines: bool,
    pub number_nonblank_lines: bool,
}

type MyResult<T> = Result<T, Box<dyn Error>>;

pub fn run(config: Config) -> MyResult<()> {
    for filename in config.files {
        match open(&filename) {
            Err(err) => eprintln!("Failed to open {}: {}", filename, err),
            Ok(handle) => {
                if config.number_lines {
                    handle
                        .lines()
                        .enumerate()
                        .for_each(|(i, l)| println!("{:>6}\t{}", i + 1, l.unwrap()))
                } else if config.number_nonblank_lines {
                    let mut i = 0;
                    for l in handle.lines() {
                        if l.as_ref().unwrap().is_empty() {
                            println!("");
                            continue;
                        } else {
                            i += 1;
                            println!("{:>6}\t{}", i, l.unwrap())
                        }
                    }
                } else {
                    handle.lines().for_each(|l| println!("{}", l.unwrap()))
                }
            }
        }
    }
    Ok(())
}

pub fn get_args() -> MyResult<Config> {
    let matches = App::new("catr")
        .version("0.1.0")
        .author("cavacado <zl@zl.com>")
        .about("rust cat")
        .arg(
            Arg::with_name("files")
                .value_name("FILE")
                .help("Input file(s)")
                .default_value("-")
                .multiple(true),
        )
        .arg(
            Arg::with_name("number_lines")
                .short("n")
                .long("number")
                .help("Number the output lines, starting at 1")
                .takes_value(false)
                .conflicts_with("number_nonblank_lines"),
        )
        .arg(
            Arg::with_name("number_nonblank_lines")
                .short("b")
                .long("number-nonblank")
                .help("Number the non-blank output lines, starting at 1")
                .takes_value(false)
                .conflicts_with("number_lines"),
        )
        .get_matches();
    Ok(Config {
        files: matches.values_of_lossy("files").unwrap(),
        number_lines: matches.is_present("number_lines"),
        number_nonblank_lines: matches.is_present("number_nonblank_lines"),
    })
}

fn open(filename: &str) -> MyResult<Box<dyn BufRead>> {
    match filename {
        "-" => Ok(Box::new(BufReader::new(io::stdin()))),
        _ => Ok(Box::new(BufReader::new(File::open(filename)?))),
    }
}

