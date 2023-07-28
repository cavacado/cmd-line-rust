use clap::{App, Arg};
use core::fmt;
use rand::seq::SliceRandom;
use rand::SeedableRng;
use regex::{Regex, RegexBuilder};
use std::error::Error;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use walkdir::WalkDir;

type MyResult<T> = Result<T, Box<dyn Error>>;

#[derive(Debug)]
pub struct Config {
    sources: Vec<String>,
    pattern: Option<Regex>,
    seed: Option<u64>,
}

#[derive(Debug)]
struct Fortune {
    source: String,
    text: String,
}
impl fmt::Display for Fortune {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.text)
    }
}

pub fn get_args() -> MyResult<Config> {
    let matches = App::new("fortuner")
        .version("0.1.0")
        .author("zl <zl@zl.com>")
        .about("Rust fortune")
        .arg(
            Arg::with_name("sources")
                .multiple(true)
                .value_name("FILE")
                .help("Input file or directories")
                .required(true),
        )
        .arg(
            Arg::with_name("case")
                .takes_value(false)
                .short("i")
                .long("insensitive")
                .help("Prints help information"),
        )
        .arg(
            Arg::with_name("pattern")
                .short("m")
                .long("pattern")
                .value_name("PATTERN")
                .help("Pattern"),
        )
        .arg(
            Arg::with_name("seed")
                .short("s")
                .long("seed")
                .value_name("SEED")
                .help("Random seed"),
        )
        .get_matches();

    let sources = matches.values_of_lossy("sources");
    let pattern = matches
        .value_of("pattern")
        .map(|p| {
            let case = matches.is_present("case");
            match RegexBuilder::new(p).case_insensitive(case).build() {
                Ok(regex) => Ok(regex),
                Err(_) => Err(format!("Invalid --pattern \"{}\"", p)),
            }
        })
        .transpose()?;
    let seed = matches.value_of("seed").map(parse_u64).transpose()?;

    Ok(Config {
        sources: sources.unwrap(),
        pattern,
        seed,
    })
}

pub fn run(config: Config) -> MyResult<()> {
    let files = find_files(&config.sources)?;
    let fortunes = read_fortunes(&files)?;
    if fortunes.is_empty() {
        println!("No fortunes found");
    } else {
        match config.pattern {
            Some(pattern) => {
                let mut sources = Vec::new();
                for fortune in fortunes {
                    if pattern.is_match(&fortune.to_string()) {
                        println!("{}\n%", fortune);
                        sources.push(fortune.source);
                    }
                }
                sources.dedup();
                sources.iter().for_each(|s| eprintln!("({})\n%", s));
            }
            None => {
                println!("{}", pick_fortune(&fortunes, config.seed).unwrap());
            }
        }
    }
    Ok(())
}

fn parse_u64(val: &str) -> MyResult<u64> {
    match val.parse::<u64>() {
        Ok(i) => Ok(i),
        Err(_) => Err(From::from(format!("\"{}\" not a valid integer", val))),
    }
}

fn find_files(paths: &[String]) -> MyResult<Vec<PathBuf>> {
    let mut buffers = Vec::new();
    let mut seen: Vec<&String> = Vec::new();
    for p in paths {
        let path_buf = PathBuf::from(p);
        if seen.contains(&p) {
            continue;
        } else {
            seen.push(p);
        }
        let walker = WalkDir::new(path_buf);
        for entry in walker {
            match entry {
                Ok(entry) => {
                    if entry.file_type().is_file() {
                        buffers.push(entry.into_path())
                    }
                }
                Err(e) => {
                    return Err(From::from(format!(
                        "{}: {}",
                        e.path().unwrap().display(),
                        e.io_error().unwrap()
                    )))
                }
            }
        }
    }
    buffers.sort();
    Ok(buffers)
}

fn read_fortunes(paths: &[PathBuf]) -> MyResult<Vec<Fortune>> {
    let mut fortunes = Vec::new();
    for p in paths {
        let file = File::open(p)?;
        let reader = BufReader::new(file);
        for f in reader.split(b'%') {
            let chunk = f?;
            let text = String::from_utf8_lossy(&chunk);
            let text = match text.strip_prefix('\n') {
                Some(t) => match t.strip_suffix('\n') {
                    Some(t) => t.to_string(),
                    None => t.to_string(),
                },
                None => match text.strip_suffix('\n') {
                    Some(t) => t.to_string(),
                    None => text.to_string(),
                },
            };
            if !text.is_empty() {
                fortunes.push(Fortune {
                    source: p.file_name().unwrap().to_string_lossy().to_string(),
                    text,
                })
            }
        }
    }
    Ok(fortunes)
}

fn pick_fortune(fortunes: &[Fortune], seed: Option<u64>) -> Option<String> {
    match seed {
        Some(s) => fortunes
            .choose(&mut rand::rngs::StdRng::seed_from_u64(s))
            .map(|f| format!("{}", f)),
        None => fortunes
            .choose(&mut rand::thread_rng())
            .map(|f| format!("{}", f)),
    }
}

#[cfg(test)]
mod tests {
    use super::{find_files, parse_u64, pick_fortune, read_fortunes, Fortune};
    use std::path::PathBuf;
    #[test]
    fn test_parse_u64() {
        let res = parse_u64("a");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "\"a\" not a valid integer");
        let res = parse_u64("0");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), 0);
        let res = parse_u64("4");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), 4);
    }
    #[test]
    fn test_find_files() {
        // Verify that the function finds a file known to exist
        let res = find_files(&["./tests/inputs/jokes".to_string()]);
        assert!(res.is_ok());
        let files = res.unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(
            files.get(0).unwrap().to_string_lossy(),
            "./tests/inputs/jokes"
        );
        // Fails to find a bad file
        let res = find_files(&["/path/does/not/exist".to_string()]);
        assert!(res.is_err());
        // Finds all the input files, excludes ".dat"
        let res = find_files(&["./tests/inputs".to_string()]);
        assert!(res.is_ok());
        // Check number and order of files
        let files = res.unwrap();
        assert_eq!(files.len(), 5);
        let first = files.get(0).unwrap().display().to_string();
        assert!(first.contains("ascii-art"));
        let last = files.last().unwrap().display().to_string();
        assert!(last.contains("quotes"));
        // Test for multiple sources, path must be unique and sorted
        let res = find_files(&[
            "./tests/inputs/jokes".to_string(),
            "./tests/inputs/ascii-art".to_string(),
            "./tests/inputs/jokes".to_string(),
        ]);
        assert!(res.is_ok());
        let files = res.unwrap();
        assert_eq!(files.len(), 2);
        if let Some(filename) = files.first().unwrap().file_name() {
            assert_eq!(filename.to_string_lossy(), "ascii-art".to_string())
        }
        if let Some(filename) = files.last().unwrap().file_name() {
            assert_eq!(filename.to_string_lossy(), "jokes".to_string())
        }
    }
    #[test]
    fn test_read_fortunes() {
        // One input file
        let res = read_fortunes(&[PathBuf::from("./tests/inputs/jokes")]);
        assert!(res.is_ok());
        if let Ok(fortunes) = res {
            // Correct number and sorting assert_eq!(fortunes.len(), 6);
            assert_eq!(
                fortunes.first().unwrap().text,
                "Q. What do you call a head of lettuce in a shirt and tie?\n\
                A. Collared greens."
            );
            assert_eq!(
                fortunes.last().unwrap().text,
                "Q: What do you call a deer wearing an eye patch?\n\
                A: A bad idea (bad-eye deer)."
            );
        }
        // Multiple input files
        let res = read_fortunes(&[
            PathBuf::from("./tests/inputs/jokes"),
            PathBuf::from("./tests/inputs/quotes"),
        ]);
        assert!(res.is_ok());
        assert_eq!(res.unwrap().len(), 11);
    }
    #[test]
    fn test_pick_fortune() {
        // Create a slice of fortunes
        let fortunes = &[
            Fortune {
                source: "fortunes".to_string(),
                text: "You cannot achieve the impossible without \
                          attempting the absurd."
                    .to_string(),
            },
            Fortune {
                source: "fortunes".to_string(),
                text: "Assumption is the mother of all screw-ups.".to_string(),
            },
            Fortune {
                source: "fortunes".to_string(),
                text: "Neckties strangle clear thinking.".to_string(),
            },
        ];
        // Pick a fortune with a seed
        assert_eq!(
            pick_fortune(fortunes, Some(1)).unwrap(),
            "Neckties strangle clear thinking.".to_string()
        );
    }
}
