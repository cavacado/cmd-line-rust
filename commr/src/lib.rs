use clap::{App, Arg};
use std::cmp;
use std::error::Error;
use std::fs::File;
use std::io::{self, BufRead, BufReader};

type MyResult<T> = Result<T, Box<dyn Error>>;

#[derive(Debug)]
pub struct Config {
    file1: String,
    file2: String,
    show_col1: bool,
    show_col2: bool,
    show_col3: bool,
    insensitive: bool,
    delimiter: String,
}

pub fn get_args() -> MyResult<Config> {
    let matches = App::new("commr")
        .version("0.1.0")
        .author("zl <zl@zl.com")
        .about("Rust comm")
        .arg(
            Arg::with_name("file1")
                .value_name("FILE1")
                .help("Input file 1")
                .required(true),
        )
        .arg(
            Arg::with_name("file2")
                .value_name("FILE2")
                .help("Input file 2")
                .required(true),
        )
        .arg(
            Arg::with_name("c1")
                .short("1")
                .takes_value(false)
                .help("Suppress printing of column 1"),
        )
        .arg(
            Arg::with_name("c2")
                .short("2")
                .takes_value(false)
                .help("Suppress printing of column 2"),
        )
        .arg(
            Arg::with_name("c3")
                .short("3")
                .takes_value(false)
                .help("Suppress printing of column 3"),
        )
        .arg(
            Arg::with_name("case")
                .short("i")
                .takes_value(false)
                .help("Case-insensitive comparison of lines"),
        )
        .arg(
            Arg::with_name("delimiter")
                .short("d")
                .long("output-delimiter")
                .value_name("DELIM")
                .default_value("\t"),
        )
        .get_matches();

    let file1 = matches.value_of("file1").unwrap();
    let file2 = matches.value_of("file2").unwrap();
    let show_col1 = !matches.is_present("c1");
    let show_col2 = !matches.is_present("c2");
    let show_col3 = !matches.is_present("c3");
    let insensitive = matches.is_present("case");
    let delimiter = matches.value_of("delimiter").unwrap_or_default();

    Ok(Config {
        file1: String::from(file1),
        file2: String::from(file2),
        show_col1,
        show_col2,
        show_col3,
        insensitive,
        delimiter: String::from(delimiter),
    })
}

enum Column<'a> {
    Col1(&'a str),
    Col2(&'a str),
    Col3(&'a str),
}
pub fn run(config: Config) -> MyResult<()> {
    let print = |col: Column| {
        let mut columns = vec![];
        match col {
            Column::Col1(val) => {
                if config.show_col1 {
                    columns.push(val);
                }
            }
            Column::Col2(val) => {
                if config.show_col2 {
                    if config.show_col1 {
                        columns.push("");
                    }
                    columns.push(val);
                }
            }
            Column::Col3(val) => {
                if config.show_col3 {
                    if config.show_col1 {
                        columns.push("");
                    }
                    if config.show_col2 {
                        columns.push("");
                    }
                    columns.push(val);
                }
            }
        };
        if !columns.is_empty() {
            println!("{}", columns.join(&config.delimiter));
        }
    };
    let file1 = &config.file1;
    let file2 = &config.file2;
    if file1 == "-" && file2 == "-" {
        return Err(From::from("Both input files cannot be STDIN (\"-\")"));
    }
    let case = |line: String| {
        if config.insensitive {
            line.to_lowercase()
        } else {
            line
        }
    };
    let mut lines1 = open(&file1)?.lines().filter_map(Result::ok).map(case);
    let mut lines2 = open(&file2)?.lines().filter_map(Result::ok).map(case);
    let mut line1 = lines1.next();
    let mut line2 = lines2.next();
    while line1.is_some() || line2.is_some() {
        match (&line1, &line2) {
            (Some(c1), Some(c2)) => match c1.cmp(c2) {
                cmp::Ordering::Equal => {
                    print(Column::Col3(c1));
                    line1 = lines1.next();
                    line2 = lines2.next();
                }
                cmp::Ordering::Less => {
                    print(Column::Col1(c1));
                    line1 = lines1.next();
                }
                cmp::Ordering::Greater => {
                    print(Column::Col2(c2));
                    line2 = lines2.next();
                }
            },
            (Some(c1), None) => {
                print(Column::Col1(c1));
                line1 = lines1.next();
            }
            (None, Some(c2)) => {
                print(Column::Col2(c2));
                line2 = lines2.next();
            }
            (None, None) => {
                unreachable!()
            }
        }
    }
    Ok(())
}

pub fn open(filename: &str) -> MyResult<Box<dyn BufRead>> {
    match filename {
        "-" => Ok(Box::new(BufReader::new(io::stdin()))),
        _ => Ok(Box::new(BufReader::new(
            File::open(filename).map_err(|e| format!("{}: {}", filename, e))?,
        ))),
    }
}
