use chrono::{DateTime, Utc};
use clap::{App, Arg};
use std::{error::Error, fs, os::unix::prelude::MetadataExt, path::PathBuf};
use tabular::{Row, Table};

type MyResult<T> = Result<T, Box<dyn Error>>;

#[derive(Debug)]
pub struct Config {
    paths: Vec<String>,
    long: bool,
    show_hidden: bool,
}

pub fn get_args() -> MyResult<Config> {
    let matches = App::new("lsr")
        .version("0.1.0")
        .author("zl <zl@zl.com>")
        .about("Rust ls")
        .arg(
            Arg::with_name("all")
                .long("all")
                .short("a")
                .takes_value(false)
                .help("Show all files"),
        )
        .arg(
            Arg::with_name("long")
                .short("l")
                .long("long")
                .help("Long listing")
                .takes_value(false),
        )
        .arg(
            Arg::with_name("files")
                .value_name("PATH")
                .help("Files and/or directories")
                .default_value(".")
                .multiple(true),
        )
        .get_matches();
    let paths = matches.values_of_lossy("files");
    let long = matches.is_present("long");
    let show_hidden = matches.is_present("all");

    Ok(Config {
        paths: paths.unwrap(),
        long,
        show_hidden,
    })
}

pub fn run(config: Config) -> MyResult<()> {
    let paths = find_files(&config.paths, config.show_hidden)?;
    if config.long {
        println!("{}", format_output(&paths)?);
    } else {
        for path in paths {
            println!("{}", path.display());
        }
    }
    Ok(())
}

pub fn find_files(paths: &[String], show_hidden: bool) -> MyResult<Vec<PathBuf>> {
    let mut res = Vec::new();
    for path in paths {
        let metadata = fs::metadata(path);
        match metadata {
            Ok(meta) => {
                if meta.is_file() {
                    res.push(PathBuf::from(path));
                } else {
                    for entry in fs::read_dir(path)? {
                        let entry = entry?;
                        let entry_path = entry.path().to_string_lossy().to_string();
                        if !show_hidden && entry.file_name().to_string_lossy().starts_with(".") {
                            continue;
                        } else {
                            res.push(PathBuf::from(entry_path))
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("{}: {}", path, e);
            }
        }
    }
    Ok(res)
}

fn format_output(paths: &[PathBuf]) -> MyResult<String> {
    let fmt = "{:<}{:<}  {:>}  {:<}  {:<}  {:>}  {:<}  {:<}";
    let mut table = Table::new(fmt);
    for path in paths {
        let metadata = path.metadata()?;
        let username = users::get_user_by_uid(metadata.uid())
            .map(|user| user.name().to_string_lossy().into_owned())
            .unwrap();
        let grpname = users::get_group_by_gid(metadata.gid())
            .map(|grp| grp.name().to_string_lossy().into_owned())
            .unwrap();
        let modified: DateTime<Utc> = From::from(metadata.modified()?);
        let mode = format_mode(metadata.mode());

        table.add_row(
            Row::new()
                .with_cell(if metadata.is_dir() { "d" } else { "-" })
                .with_cell(mode)
                .with_cell(metadata.nlink())
                .with_cell(username)
                .with_cell(grpname)
                .with_cell(metadata.size())
                .with_cell(modified.format("%b %d %g"))
                .with_cell(path.display()),
        );
    }
    Ok(format!("{}", table))
}

fn format_chunk((r, w, x): (u32, u32, u32)) -> String {
    let r = if r != 0 { "r" } else { "-" };
    let w = if w != 0 { "w" } else { "-" };
    let x = if x != 0 { "x" } else { "-" };
    format!("{}{}{}", r, w, x)
}

fn format_mode(mode: u32) -> String {
    let user = (mode & 0o400, mode & 0o200, mode & 0o100);
    let grp = (mode & 0o040, mode & 0o020, mode & 0o010);
    let others = (mode & 0o004, mode & 0o002, mode & 0o001);
    format!(
        "{}{}{}",
        format_chunk(user),
        format_chunk(grp),
        format_chunk(others)
    )
}

#[cfg(test)]
mod test {
    use super::{find_files, format_mode, format_output};
    use std::path::PathBuf;
    fn long_match(
        line: &str,
        expected_name: &str,
        expected_perms: &str,
        expected_size: Option<&str>,
    ) {
        let parts: Vec<_> = line.split_whitespace().collect();
        assert!(parts.len() > 0 && parts.len() <= 10);
        let perms = parts.get(0).unwrap();
        assert_eq!(perms, &expected_perms);
        if let Some(size) = expected_size {
            let file_size = parts.get(4).unwrap();
            assert_eq!(file_size, &size);
        }
        let display_name = parts.last().unwrap();
        assert_eq!(display_name, &expected_name);
    }
    #[test]
    fn test_find_files() {
        // Find all nonhidden entries in a directory
        let res = find_files(&["tests/inputs".to_string()], false);
        assert!(res.is_ok());
        let mut filenames: Vec<_> = res
            .unwrap()
            .iter()
            .map(|entry| entry.display().to_string())
            .collect();
        filenames.sort();
        assert_eq!(
            filenames,
            [
                "tests/inputs/bustle.txt",
                "tests/inputs/dir",
                "tests/inputs/empty.txt",
                "tests/inputs/fox.txt",
            ]
        );
        // Find all entries in a directory
        let res = find_files(&["tests/inputs".to_string()], true);
        assert!(res.is_ok());
        let mut filenames: Vec<_> = res
            .unwrap()
            .iter()
            .map(|entry| entry.display().to_string())
            .collect();
        filenames.sort();
        assert_eq!(
            filenames,
            [
                "tests/inputs/.hidden",
                "tests/inputs/bustle.txt",
                "tests/inputs/dir",
                "tests/inputs/empty.txt",
                "tests/inputs/fox.txt",
            ]
        );
        // Any existing file should be found even if hidden
        let res = find_files(&["tests/inputs/.hidden".to_string()], false);
        assert!(res.is_ok());
        let filenames: Vec<_> = res
            .unwrap()
            .iter()
            .map(|entry| entry.display().to_string())
            .collect();
        assert_eq!(filenames, ["tests/inputs/.hidden"]);
        // Test multiple path arguments
        let res = find_files(
            &[
                "tests/inputs/bustle.txt".to_string(),
                "tests/inputs/dir".to_string(),
            ],
            false,
        );
        assert!(res.is_ok());
        let mut filenames: Vec<_> = res
            .unwrap()
            .iter()
            .map(|entry| entry.display().to_string())
            .collect();
        filenames.sort();
        assert_eq!(
            filenames,
            ["tests/inputs/bustle.txt", "tests/inputs/dir/spiders.txt"]
        );
    }
    #[test]
    fn test_find_files_hidden() {
        let res = find_files(&["tests/inputs".to_string()], true);
        assert!(res.is_ok());
        let mut filenames: Vec<_> = res
            .unwrap()
            .iter()
            .map(|entry| entry.display().to_string())
            .collect();
        filenames.sort();
        assert_eq!(
            filenames,
            [
                "tests/inputs/.hidden",
                "tests/inputs/bustle.txt",
                "tests/inputs/dir",
                "tests/inputs/empty.txt",
                "tests/inputs/fox.txt",
            ]
        );
    }
    #[test]
    fn test_format_mode() {
        assert_eq!(format_mode(0o755), "rwxr-xr-x");
        assert_eq!(format_mode(0o421), "r---w---x");
    }

    #[test]
    fn test_format_output_one() {
        let bustle_path = "tests/inputs/bustle.txt";
        let bustle = PathBuf::from(bustle_path);
        let res = format_output(&[bustle]);
        assert!(res.is_ok());
        let out = res.unwrap();
        let lines: Vec<&str> = out.split("\n").filter(|s| !s.is_empty()).collect();
        assert_eq!(lines.len(), 1);
        let line1 = lines.first().unwrap();
        long_match(&line1, bustle_path, "-rw-r--r--", Some("193"));
    }

    #[test]
    fn test_format_output_two() {
        let res = format_output(&[
            PathBuf::from("tests/inputs/dir"),
            PathBuf::from("tests/inputs/empty.txt"),
        ]);
        assert!(res.is_ok());
        let out = res.unwrap();
        let mut lines: Vec<&str> = out.split("\n").filter(|s| !s.is_empty()).collect();
        lines.sort();
        assert_eq!(lines.len(), 2);
        let empty_line = lines.remove(0);
        long_match(
            &empty_line,
            "tests/inputs/empty.txt",
            "-rw-r--r--",
            Some("0"),
        );
        let dir_line = lines.remove(0);
        long_match(&dir_line, "tests/inputs/dir", "drwxr-xr-x", None);
    }
}
