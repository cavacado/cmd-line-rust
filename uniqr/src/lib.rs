use clap::{App, Arg};
use std::{
    error::Error,
    fs::File,
    io::{self, BufRead, BufReader, Write},
};

type MyResult<T> = Result<T, Box<dyn Error>>;

#[derive(Debug)]
pub struct Config {
    in_file: String,
    out_file: Option<String>,
    count: bool,
}

pub fn get_args() -> MyResult<Config> {
    let matches = App::new("uniqr")
        .version("0.1.0")
        .author("zl <zl@zl.com>")
        .about("Rust uniq")
        .arg(
            Arg::with_name("in_file")
                .value_name("IN_FILE")
                .help("Input file")
                .default_value("-"),
        )
        .arg(
            Arg::with_name("out_file")
                .value_name("OUT_FILE")
                .help("Output file"),
        )
        .arg(
            Arg::with_name("count")
                .value_name("count")
                .long("count")
                .short("c")
                .takes_value(false)
                .help("Show counts"),
        )
        .get_matches();
    let in_file = matches
        .value_of_lossy("in_file")
        .map(|val| val.to_string())
        .unwrap();
    let out_file = matches.value_of("out_file").map(|val| val.to_string());
    let count = matches.is_present("count");
    Ok(Config {
        in_file,
        out_file,
        count,
    })
}

pub fn run(config: Config) -> MyResult<()> {
    let mut file = open(&config.in_file).map_err(|e| format!("{}: {}", config.in_file, e))?;
    let mut out_file: Box<dyn Write> = match &config.out_file {
        Some(out) => Box::new(File::create(out)?),
        None => Box::new(io::stdout()),
    };
    let mut line = String::new();
    let mut line_vec: Vec<String> = Vec::new();
    loop {
        let bytes = file.read_line(&mut line)?;
        if bytes == 0 {
            break;
        }
        line_vec.push(line.clone());
        line.clear();
    }
    let blank = String::new();
    let mut res: Vec<Vec<&String>> = Vec::new();
    let mut prev = line_vec.get(0).unwrap_or(&blank);
    let mut temp: Vec<&String> = Vec::new();
    temp.push(prev);
    for line in line_vec.iter().skip(1) {
        let t1: String = prev.chars().filter(|c| !c.is_whitespace()).collect();
        let t2: String = line.chars().filter(|c| !c.is_whitespace()).collect();
        if t1.eq(&t2) {
            temp.push(line);
        } else {
            res.push(temp);
            temp = Vec::new();
            temp.push(line)
        }
        prev = line;
    }
    res.push(temp);
    res.iter()
        .map(|seq| {
            let count = seq.len();
            let line = seq[0].clone();
            (count, line)
        })
        .for_each(|el| {
            if el.1.eq(&blank) {
                print!("")
            } else if config.count {
                write!(&mut out_file, "{:>4} {}", el.0, el.1).unwrap();
            } else {
                write!(&mut out_file, "{}", el.1).unwrap();
            }
        });
        // note here that my solution is very diff from the book's one
        // mine is really less efficient, but i rather work with a 
        // vector collection than directly on the line of bytes.
        // maybe 1 day, when I come back with a fresh pair of eyes
        // can try it again.
    Ok(())
}

fn open(filename: &str) -> MyResult<Box<dyn BufRead>> {
    match filename {
        "-" => Ok(Box::new(BufReader::new(io::stdin()))),
        _ => Ok(Box::new(BufReader::new(File::open(filename)?))),
    }
}
