use clap::{App, Arg};
use std::error::Error;
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Seek};

type MyResult<T> = Result<T, Box<dyn Error>>;

#[derive(Debug, PartialEq)]
pub enum TakeValue {
    PlusZero,
    TakeNum(i64),
}

#[derive(Debug)]
pub struct Config {
    files: Vec<String>,
    lines: TakeValue,
    bytes: Option<TakeValue>,
    quiet: bool,
}

pub fn get_args() -> MyResult<Config> {
    let matches = App::new("tailr")
        .version("0.1.0")
        .about("Rust tail")
        .author("zl <zl@zl.com>")
        .arg(
            Arg::with_name("files")
                .value_name("FILE")
                .help("Input file(s)")
                .required(true)
                .multiple(true),
        )
        .arg(
            Arg::with_name("lines")
                .short("n")
                .long("lines")
                .value_name("LINES")
                .help("Number of lines")
                .default_value("-10"),
        )
        .arg(
            Arg::with_name("bytes")
                .short("c")
                .long("bytes")
                .value_name("BYTES")
                .help("Number of bytes")
                .conflicts_with("lines"),
        )
        .arg(
            Arg::with_name("quiet")
                .short("q")
                .long("quiet")
                .help("Suppress headers")
                .takes_value(false),
        )
        .get_matches();

    let files = matches.values_of_lossy("files");
    let lines = matches
        .value_of("lines")
        .map(parse_num)
        .transpose()
        .map_err(|e| format!("illegal line count -- {}", e))?;

    let bytes = matches
        .value_of("bytes")
        .map(parse_num)
        .transpose()
        .map_err(|e| format!("illegal byte count -- {}", e))?;
    let quiet = matches.is_present("quiet");

    Ok(Config {
        files: files.unwrap(),
        lines: lines.unwrap(),
        bytes: bytes,
        quiet,
    })
}

pub fn parse_num(val: &str) -> MyResult<TakeValue> {
    match val {
        "+0" => Ok(TakeValue::PlusZero),
        other => {
            if other.starts_with("+") {
                match other.strip_prefix("+").unwrap().parse::<i64>() {
                    Ok(val) => Ok(TakeValue::TakeNum(val)),
                    Err(_) => return Err(other.into()),
                }
            } else if other.starts_with("-") {
                match other.parse::<i64>() {
                    Ok(val) => Ok(TakeValue::TakeNum(val)),
                    Err(_) => return Err(other.into()),
                }
            } else {
                match other.parse::<i64>() {
                    Ok(val) => Ok(TakeValue::TakeNum(-val)),
                    Err(_) => return Err(other.into()),
                }
            }
        }
    }
}

pub fn run(config: Config) -> MyResult<()> {
    let num_files = config.files.len();
    for (file_num, filename) in config.files.iter().enumerate() {
        match File::open(&filename) {
            Err(err) => eprintln!("{}: {}", filename, err),
            Ok(handle) => {
                if !config.quiet && num_files > 1 {
                    println!(
                        "{}==> {} <==",
                        if file_num > 0 { "\n" } else { "" },
                        filename
                    );
                }
                let (lc, bc) = count_lines_bytes(&filename)?;
                if let Some(bytes) = &config.bytes {
                    print_bytes(handle, bytes, bc)?;
                } else {
                    let handle = BufReader::new(handle);
                    print_lines(handle, &config.lines, lc)?;
                }
            }
        }
    }
    Ok(())
}

pub fn count_lines_bytes(filename: &str) -> MyResult<(i64, i64)> {
    let mut file = BufReader::new(File::open(filename)?);
    let mut line_num = 0;
    let mut byte_num = 0;
    let mut buffer = Vec::new();
    loop {
        let bytes_read = file.read_until(b'\n', &mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        line_num += 1;
        byte_num += bytes_read as i64;
        buffer.clear();
    }
    Ok((line_num, byte_num))
}

pub fn print_lines(
    mut file: impl BufRead,
    num_lines: &TakeValue,
    total_lines: i64,
) -> MyResult<()> {
    let idx = get_start_index(num_lines, total_lines);
    match idx {
        Some(idx) => {
            let mut line_num = 0;
            let mut buf = Vec::new();
            loop {
                let bytes_read = file.read_until(b'\n', &mut buf)?;
                if bytes_read == 0 {
                    break;
                }
                if line_num >= idx {
                    print!("{}", String::from_utf8_lossy(&buf));
                }
                line_num += 1;
                buf.clear();
            }
            Ok(())
        }
        None => Ok(()),
    }
}

fn print_bytes<T: Read + Seek>(
    mut file: T,
    num_bytes: &TakeValue,
    total_bytes: i64,
) -> MyResult<()> {
    let idx = get_start_index(num_bytes, total_bytes);
    match idx {
        Some(idx) => {
            let mut buffer = Vec::new();
            file.seek(std::io::SeekFrom::Start(idx))?;
            file.read_to_end(&mut buffer)?;
            if !buffer.is_empty() {
                print!("{}", String::from_utf8_lossy(&buffer));
            }
            Ok(())
        }
        None => Ok(()),
    }
}

fn get_start_index(take_val: &TakeValue, total: i64) -> Option<u64> {
    match take_val {
        TakeValue::PlusZero => {
            if total > 0 {
                Some(0)
            } else {
                None
            }
        }
        TakeValue::TakeNum(0) => None,
        TakeValue::TakeNum(x) if x < &0_i64 => {
            if total + x > 0 {
                Some((total + x).try_into().unwrap())
            } else {
                Some(0)
            }
        }
        TakeValue::TakeNum(rest) => {
            if total < *rest {
                None
            } else {
                Some((rest - 1).try_into().unwrap())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{count_lines_bytes, get_start_index, parse_num, TakeValue::*};
    #[test]
    fn test_parse_num() {
        // All integers should be interpreted as negative numbers
        let res = parse_num("3");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), TakeNum(-3));
        // A leading "+" should result in a positive number
        let res = parse_num("+3");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), TakeNum(3));
        // An explicit "-" value should result in a negative number
        let res = parse_num("-3");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), TakeNum(-3));
        // Zero is zero
        let res = parse_num("0");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), TakeNum(0));
        // Plus zero is special
        let res = parse_num("+0");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), PlusZero);
        // Test boundaries
        let res = parse_num(&i64::MAX.to_string());
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), TakeNum(i64::MIN + 1));
        let res = parse_num(&(i64::MIN + 1).to_string());
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), TakeNum(i64::MIN + 1));
        let res = parse_num(&format!("+{}", i64::MAX));
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), TakeNum(i64::MAX));
        let res = parse_num(&i64::MIN.to_string());
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), TakeNum(i64::MIN));
        // A floating-point value is invalid
        let res = parse_num("3.14");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "3.14");
        // Any noninteger string is invalid
        let res = parse_num("foo");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "foo");
    }
    #[test]
    fn test_count_lines_bytes() {
        let res = count_lines_bytes("tests/inputs/one.txt");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), (1, 24));
        let res = count_lines_bytes("tests/inputs/ten.txt");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), (10, 49));
    }
    #[test]
    fn test_get_start_index() {
        // +0 from an empty file (0 lines/bytes) returns None
        assert_eq!(get_start_index(&PlusZero, 0), None);
        // +0 from a nonempty file returns an index that // is one less than the number of lines/bytes
        assert_eq!(get_start_index(&PlusZero, 1), Some(0));
        // Taking 0 lines/bytes returns None
        assert_eq!(get_start_index(&TakeNum(0), 1), None);
        // Taking any lines/bytes from an empty file returns None
        assert_eq!(get_start_index(&TakeNum(1), 0), None);
        // Taking more lines/bytes than is available returns None
        assert_eq!(get_start_index(&TakeNum(2), 1), None);
        // When starting line/byte is less than total lines/bytes,
        // return one less than starting number
        assert_eq!(get_start_index(&TakeNum(1), 10), Some(0));
        assert_eq!(get_start_index(&TakeNum(2), 10), Some(1));
        assert_eq!(get_start_index(&TakeNum(3), 10), Some(2));
        // When starting line/byte is negative and less than total,
        // return total - start
        assert_eq!(get_start_index(&TakeNum(-1), 10), Some(9));
        assert_eq!(get_start_index(&TakeNum(-2), 10), Some(8));
        assert_eq!(get_start_index(&TakeNum(-3), 10), Some(7));
        // When starting line/byte is negative and more than total,
        // return 0 to print the whole file
        assert_eq!(get_start_index(&TakeNum(-20), 10), Some(0));
    }
}
