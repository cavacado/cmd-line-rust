use clap::{App, Arg};
use regex::Regex;
use std::error::Error;
use walkdir::{DirEntry, WalkDir};

type MyResult<T> = Result<T, Box<dyn Error>>;

#[derive(Debug, Eq, PartialEq)]
enum EntryType {
    Dir,
    File,
    Link,
}

#[derive(Debug)]
pub struct Config {
    paths: Vec<String>,
    names: Vec<Regex>,
    entry_types: Vec<EntryType>,
}

pub fn get_args() -> MyResult<Config> {
    let matches = App::new("findr")
        .version("0.1.0")
        .author("zl <zl@zl.com>")
        .about("Rust find")
        .arg(
            Arg::with_name("names")
                .value_name("NAME")
                .help("Name")
                .short("n")
                .long("name")
                .takes_value(true)
                .multiple(true),
        )
        .arg(
            Arg::with_name("type")
                .value_name("TYPE")
                .help("Entry type")
                .short("t")
                .long("type")
                .multiple(true)
                .possible_values(&["f", "d", "l"]),
        )
        .arg(
            Arg::with_name("paths")
                .value_name("PATH")
                .help("Search paths")
                .multiple(true)
                .default_value("."),
        )
        .get_matches();

    let names = matches
        .values_of_lossy("names")
        .map(|vals| {
            vals.into_iter()
                .map(|name| Regex::new(&name).map_err(|_| format!("Invalid --name \"{}\"", name)))
                .collect::<Result<Vec<_>, _>>()
        })
        .transpose()?
        .unwrap_or_default();

    // stopped here, somehting wrong with names
    // seems like its better not to pull out values before
    // mapping or processing the data within.
    // alot of friction when trying to do so.
    // let names = match matches.values_of_lossy("name") {
    //     Some(ns) => ns
    //         .iter()
    //         .map(|name| Regex::new(name).map_err(|_| format!("Invalid --name {}", name)))
    //         .collect(),
    //     None => Vec::new(),
    // };

    let paths = matches
        .values_of_lossy("paths")
        .expect("paniked at parsing paths");

    let entry_types = match matches.values_of_lossy("type") {
        Some(ts) => ts
            .iter()
            .map(|t| match t.as_str() {
                "f" => EntryType::File,
                "d" => EntryType::Dir,
                "l" => EntryType::Link,
                _ => unreachable!("Invalid type"),
            })
            .collect(),
        None => Vec::new(),
    };

    Ok(Config {
        paths,
        names,
        entry_types,
    })
}

pub fn run(config: Config) -> MyResult<()> {
    let cb = |names: &Vec<Regex>, entry: &DirEntry| {
        let names = names.clone();
        match names.len() {
            0 => println!("{}", entry.path().display()),
            _ => names.iter().for_each(|re| {
                if re.is_match(entry.file_name().to_str().unwrap()) {
                    println!("{}", entry.path().display())
                }
            }),
        }
    };
    for path in config.paths {
        for entry in WalkDir::new(path) {
            match entry {
                Err(e) => eprintln!("{}", e),
                Ok(entry) => match config.entry_types.len() {
                    0 => cb(&config.names, &entry),
                    _ => config.entry_types.iter().for_each(|t| match t {
                        EntryType::Dir => {
                            if entry.file_type().is_dir() {
                                cb(&config.names, &entry);
                            }
                        }
                        EntryType::File => {
                            if entry.file_type().is_file() {
                                cb(&config.names, &entry);
                            }
                        }
                        EntryType::Link => {
                            if entry.path().is_symlink() {
                                cb(&config.names, &entry);
                            }
                        }
                    }),
                },
            }
        }
    }
    // println!("{:#?}", config);
    Ok(())
}
