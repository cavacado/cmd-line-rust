use crate::Extract::*;
use clap::{App, Arg};
use csv::{ReaderBuilder, StringRecord, WriterBuilder};
use regex::Regex;
use std::{
    error::Error,
    fs::File,
    io::{self, BufRead, BufReader},
    ops::Range,
};

type MyResult<T> = Result<T, Box<dyn Error>>;
type PositionList = Vec<Range<usize>>;

#[derive(Debug)]
pub enum Extract {
    Fields(PositionList),
    Bytes(PositionList),
    Chars(PositionList),
}

#[derive(Debug)]
pub struct Config {
    files: Vec<String>,
    delimiter: u8,
    extract: Extract,
}

pub fn get_args() -> MyResult<Config> {
    let matches = App::new("cutr")
        .version("0.1.0")
        .author("zl <zl@zl.com>")
        .about("Rust cut")
        .arg(
            Arg::with_name("files")
                .value_name("FILE")
                .help("Input file(s)")
                .default_value("-")
                .multiple(true),
        )
        .arg(
            Arg::with_name("bytes")
                .value_name("BYTES")
                .long("bytes")
                .short("b")
                .help("Selected bytes")
                .conflicts_with_all(&["chars", "fields"]),
        )
        .arg(
            Arg::with_name("chars")
                .value_name("CHARS")
                .long("chars")
                .short("c")
                .help("Selected characters")
                .conflicts_with_all(&["bytes", "fields"]),
        )
        .arg(
            Arg::with_name("fields")
                .value_name("FIELDS")
                .long("fields")
                .short("f")
                .help("Selected fields")
                .conflicts_with_all(&["chars", "bytes"]),
        )
        .arg(
            Arg::with_name("delim")
                .value_name("DELIMITER")
                .long("delim")
                .short("d")
                .default_value("\t")
                .help("Field delimiter"),
        )
        .get_matches();

    let files = matches.values_of_lossy("files").unwrap();
    let delimiter = matches.value_of("delim").unwrap();
    let delimiter_bytes = delimiter.as_bytes();
    if delimiter_bytes.len() != 1 {
        return Err(From::from(format!(
            "--delim \"{}\" must be a single byte",
            delimiter
        )));
    }
    let extract = if matches.is_present("fields") {
        Fields(parse_pos(matches.value_of("fields").unwrap())?)
    } else if matches.is_present("bytes") {
        Bytes(parse_pos(matches.value_of("bytes").unwrap())?)
    } else if matches.is_present("chars") {
        Chars(parse_pos(matches.value_of("chars").unwrap())?)
    } else {
        return Err("Must have --fields, --bytes, or --chars".into());
    };

    Ok(Config {
        files,
        delimiter: *delimiter_bytes.first().unwrap(),
        extract,
    })
}

pub fn run(config: Config) -> MyResult<()> {
    // println!("{:#?}", &config);
    for filename in &config.files {
        match open(filename) {
            Err(err) => eprintln!("{}: {}", filename, err),
            Ok(handle) => match &config.extract {
                Bytes(rs) => {
                    for line in handle.lines() {
                        println!("{}", extract_bytes(&line?, &rs))
                    }
                }
                Chars(rs) => {
                    for line in handle.lines() {
                        println!("{}", extract_chars(&line?, &rs));
                    }
                }
                Fields(rs) => {
                    let mut reader = ReaderBuilder::new()
                        .delimiter(config.delimiter)
                        .has_headers(false)
                        .from_reader(handle);
                    let mut wtr = WriterBuilder::new()
                        .delimiter(config.delimiter)
                        .from_writer(io::stdout());
                    for record in reader.records() {
                        wtr.write_record(extract_fields(&record?, &rs))?
                    }
                }
            },
        }
    }
    Ok(())
}

pub fn parse_positive_int(val: &str) -> MyResult<usize> {
    match val.parse::<usize>() {
        Ok(n) if n > 0 => Ok(n),
        Ok(_) => Err(format!("illegal list value: \"{}\"", 0).into()),
        Err(e) => Err(format!("illegal list value: \"{}\"", e).into()),
    }
}

fn parse_pos(range: &str) -> MyResult<PositionList> {
    let split: Vec<_> = range.split(",").collect();
    let range_re = Regex::new(r"^(\d+)-(\d+)$").unwrap();
    let single_re = Regex::new(r"^(\d+)$").unwrap();
    if split.len() < 1 {
        return Err("empty string".into());
    } else {
        split
            .into_iter()
            .map(|r| {
                if range_re.is_match(r) {
                    let mut res = r.split("-");
                    let start = parse_positive_int(res.next().unwrap())?;
                    let end = parse_positive_int(res.next().unwrap())?;
                    if start >= end {
                        return Err(format!(
                            "First number in range ({}) must be lower than second number ({})",
                            start, end
                        )
                        .into());
                    }
                    Ok(start - 1..end)
                } else if single_re.is_match(r) {
                    let init = parse_positive_int(r)?;
                    Ok(init - 1..init)
                } else {
                    return Err(format!("illegal list value: \"{}\"", r).into());
                }
            })
            .collect()
    }
}

fn open(filename: &str) -> MyResult<Box<dyn BufRead>> {
    match filename {
        "-" => Ok(Box::new(BufReader::new(io::stdin()))),
        _ => Ok(Box::new(BufReader::new(File::open(filename)?))),
    }
}

fn extract_chars(line: &str, char_pos: &[Range<usize>]) -> String {
    let mut res = String::new();
    char_pos.into_iter().for_each(|r| {
        for (i, c) in line.chars().enumerate() {
            if r.contains(&i) {
                res.push(c)
            }
        }
    });
    res
}

fn extract_bytes(line: &str, byte_pos: &[Range<usize>]) -> String {
    let mut res = Vec::new();
    byte_pos.into_iter().for_each(|r| {
        for (i, b) in line.bytes().enumerate() {
            if r.contains(&i) {
                res.push(b)
            }
        }
    });
    String::from_utf8_lossy(&res).to_string()
}

fn extract_fields(record: &StringRecord, field_pos: &[Range<usize>]) -> Vec<String> {
    let mut res = Vec::new();
    field_pos.into_iter().for_each(|r| {
        for (i, rec) in record.iter().enumerate() {
            if r.contains(&i) {
                res.push(rec.into())
            }
        }
    });
    res
}

// --------------------------------------------------
#[cfg(test)]
mod unit_tests {
    use super::{extract_bytes, extract_chars, extract_fields, parse_pos};
    use csv::StringRecord;

    #[test]
    fn test_parse_pos() {
        // The empty string is an error
        assert!(parse_pos("").is_err());

        // Zero is an error
        let res = parse_pos("0");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "illegal list value: \"0\"",);

        let res = parse_pos("0-1");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "illegal list value: \"0\"",);

        // A leading "+" is an error
        let res = parse_pos("+1");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "illegal list value: \"+1\"",);

        let res = parse_pos("+1-2");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "illegal list value: \"+1-2\"",);

        let res = parse_pos("1-+2");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "illegal list value: \"1-+2\"",);

        // Any non-number is an error
        let res = parse_pos("a");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "illegal list value: \"a\"",);

        let res = parse_pos("1,a");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "illegal list value: \"a\"",);

        let res = parse_pos("1-a");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "illegal list value: \"1-a\"",);

        let res = parse_pos("a-1");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "illegal list value: \"a-1\"",);

        // Wonky ranges
        let res = parse_pos("-");
        assert!(res.is_err());

        let res = parse_pos(",");
        assert!(res.is_err());

        let res = parse_pos("1,");
        assert!(res.is_err());

        let res = parse_pos("1-");
        assert!(res.is_err());

        let res = parse_pos("1-1-1");
        assert!(res.is_err());

        let res = parse_pos("1-1-a");
        assert!(res.is_err());

        // First number must be less than second
        let res = parse_pos("1-1");
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err().to_string(),
            "First number in range (1) must be lower than second number (1)"
        );

        let res = parse_pos("2-1");
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err().to_string(),
            "First number in range (2) must be lower than second number (1)"
        );

        // All the following are acceptable
        let res = parse_pos("1");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), vec![0..1]);

        let res = parse_pos("01");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), vec![0..1]);

        let res = parse_pos("1,3");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), vec![0..1, 2..3]);

        let res = parse_pos("001,0003");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), vec![0..1, 2..3]);

        let res = parse_pos("1-3");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), vec![0..3]);

        let res = parse_pos("0001-03");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), vec![0..3]);

        let res = parse_pos("1,7,3-5");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), vec![0..1, 6..7, 2..5]);

        let res = parse_pos("15,19-20");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), vec![14..15, 18..20]);
    }

    #[test]
    fn test_extract_chars() {
        assert_eq!(extract_chars("", &[0..1]), "".to_string());
        assert_eq!(extract_chars("ábc", &[0..1]), "á".to_string());
        assert_eq!(extract_chars("ábc", &[0..1, 2..3]), "ác".to_string());
        assert_eq!(extract_chars("ábc", &[0..3]), "ábc".to_string());
        assert_eq!(extract_chars("ábc", &[2..3, 1..2]), "cb".to_string());
        assert_eq!(extract_chars("ábc", &[0..1, 1..2, 4..5]), "áb".to_string());
    }
    #[test]
    fn test_extract_bytes() {
        assert_eq!(extract_bytes("ábc", &[0..1]), "�".to_string());
        assert_eq!(extract_bytes("ábc", &[0..2]), "á".to_string());
        assert_eq!(extract_bytes("ábc", &[0..3]), "áb".to_string());
        assert_eq!(extract_bytes("ábc", &[0..4]), "ábc".to_string());
        assert_eq!(extract_bytes("ábc", &[3..4, 2..3]), "cb".to_string());
        assert_eq!(extract_bytes("ábc", &[0..2, 5..6]), "á".to_string());
    }

    #[test]
    fn test_extract_fields() {
        let rec = StringRecord::from(vec!["Captain", "Sham", "12345"]);
        assert_eq!(extract_fields(&rec, &[0..1]), &["Captain"]);
        assert_eq!(extract_fields(&rec, &[1..2]), &["Sham"]);
        assert_eq!(extract_fields(&rec, &[0..1, 2..3]), &["Captain", "12345"]);
        assert_eq!(extract_fields(&rec, &[0..1, 3..4]), &["Captain"]);
        assert_eq!(extract_fields(&rec, &[1..2, 0..1]), &["Sham", "Captain"]);
    }
}
