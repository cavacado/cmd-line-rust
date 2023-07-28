use ansi_term::Style;
use chrono::{Datelike, Duration, Local, Month, NaiveDate, Weekday};
use clap::{App, Arg};
use itertools::Itertools;
use num_traits::FromPrimitive;
use std::{collections::HashMap, error::Error};

#[derive(Debug)]
pub struct Config {
    month: Option<u32>,
    year: i32,
    today: NaiveDate,
}

type MyResult<T> = Result<T, Box<dyn Error>>;

pub fn get_args() -> MyResult<Config> {
    let matches = App::new("calr")
        .version("0.1.0")
        .author("zl <zl@zl.com>")
        .about("Rust cal")
        .arg(
            Arg::with_name("year")
                .short("y")
                .long("year")
                .takes_value(false)
                .help("Show whole current year")
                .conflicts_with("positional")
                .conflicts_with("month"),
        )
        .arg(
            Arg::with_name("month")
                .short("m")
                .help("Month name or number (1-12)")
                .value_name("MONTH"),
        )
        .arg(
            Arg::with_name("positional")
                .value_name("YEAR")
                .help("Year (1-9999)"),
        )
        .get_matches();
    let months: HashMap<&str, u32> = HashMap::from([
        (Month::January.name(), 1),
        (Month::February.name(), 2),
        (Month::March.name(), 3),
        (Month::April.name(), 4),
        (Month::May.name(), 5),
        (Month::June.name(), 6),
        (Month::July.name(), 7),
        (Month::August.name(), 8),
        (Month::September.name(), 9),
        (Month::October.name(), 10),
        (Month::November.name(), 11),
        (Month::December.name(), 12),
    ]);
    let month = match (
        matches.is_present("month"),
        matches.is_present("positional"),
    ) {
        (true, false) | (true, true) => {
            let month = matches.value_of("month").unwrap();
            match month.parse::<u32>() {
                Ok(month) if month > 0 && month < 13 => Some(month),
                Ok(_) => {
                    return Err(From::from(format!(
                        "month \"{}\" not in the range 1 through 12",
                        month
                    )))
                }
                Err(_) => {
                    let reduced = matches.value_of("month").map(|m| {
                        let mut res = Vec::new();
                        for (k, v) in months {
                            let k = k.to_ascii_lowercase();
                            let m = m.to_ascii_lowercase();
                            if k.starts_with(&m) {
                                res.push(v)
                            }
                        }
                        (res, m)
                    });
                    match reduced {
                        Some((x, m)) => {
                            if x.len() > 1 {
                                return Err(From::from(format!("Invalid month \"{}\"", m)));
                            } else {
                                let val = x.first();
                                match val {
                                    Some(val) => Some(*val),
                                    None => {
                                        return Err(From::from(format!("Invalid month \"{}\"", m)))
                                    }
                                }
                            }
                        }
                        None => None,
                    }
                }
            }
        }
        (false, true) => None,
        (false, false) if matches.is_present("year") => None,
        (false, false) => Some(Local::now().month()),
    };
    let year = match (matches.is_present("year"), matches.value_of("positional")) {
        (false, Some(y)) => match y.parse::<i32>() {
            Ok(x) if x > 9999 || x < 1 => {
                return Err(From::from(format!(
                    "year \"{}\" not in the range 1 through 9999",
                    y
                )))
            }
            Ok(x) => x,
            Err(_) => return Err(From::from(format!("Invalid integer \"{}\"", y))),
        },
        (false, None) | (true, None) => Local::now().year(),
        (true, Some(_)) => unreachable!(),
    };

    Ok(Config {
        month,
        year,
        today: Local::now().date_naive(),
    })
}

pub fn run(config: Config) -> MyResult<()> {
    match config.month {
        Some(month) => {
            let lines = format_month(config.year, month, true, config.today);
            println!("{}", lines.join("\n"));
        }
        None => {
            let year_hdr = format!("{:^66}", config.year);
            let mnth_range: Vec<u32> = (1..13).collect();
            let mnths: Vec<Vec<String>> = mnth_range
                .iter()
                .map(|mth| format_month(config.year, *mth, false, config.today))
                .collect();
            println!("{}", year_hdr);
            for chunk in mnths.chunks(3) {
                let zipped = itertools::izip!(&chunk[0], &chunk[1], &chunk[2]);
                for lines in zipped {
                    println!("{}", format!("{}{}{}", lines.0, lines.1, lines.2));
                }
            }
        }
    }
    Ok(())
}

fn format_month(year: i32, month: u32, print_year: bool, today: NaiveDate) -> Vec<String> {
    let mut res: HashMap<u32, Vec<(u32, Weekday, NaiveDate)>> = HashMap::new();
    let date = NaiveDate::from_ymd_opt(year, month, 1);
    let last_day = last_day_in_month(year, month);
    let month = date
        .map(|d| Month::from_u32(d.month()).map(|d| d.name()))
        .flatten();
    let year = date.map(|d| d.year());
    let mut tuple_dates = Vec::new();
    if let Some(d) = date {
        let dates = d
            .iter_days()
            .take_while(|d| *d != last_day + Duration::days(1))
            .map(|d| {
                let iso_week = d.iso_week().week();
                let weekday = d.weekday();
                let day = d.day();
                (day, iso_week, weekday, d)
            });
        for (day, week, weekday, d) in dates {
            match weekday {
                Weekday::Sun => {
                    let next_wk = week + 1;
                    if res.contains_key(&next_wk) {
                        res.entry(next_wk)
                            .and_modify(|dates| dates.push((day, weekday, d)));
                    } else {
                        let mut vec = Vec::new();
                        vec.push((day, weekday, d));
                        res.insert(next_wk, vec);
                    }
                }
                _ => {
                    if res.contains_key(&week) {
                        res.entry(week)
                            .and_modify(|dates| dates.push((day, weekday, d)));
                    } else {
                        let mut vec = Vec::new();
                        vec.push((day, weekday, d));
                        res.insert(week, vec);
                    }
                }
            }
        }
        for (_, dates) in res
            .iter()
            .sorted_by(|a, b| Ord::cmp(a.0, b.0))
            .collect_vec()
        {
            tuple_dates.push(dates);
        }
    }
    let header = format!(
        "{:22}",
        format!(
            "{:^20}",
            format!(
                "{} {}",
                month.unwrap(),
                if print_year {
                    year.unwrap().to_string()
                } else {
                    "".to_string()
                }
            )
        )
    );
    let weekday_hdrs = format!("Su Mo Tu We Th Fr Sa  ");

    let mut print_res = Vec::new();
    print_res.push(header);
    print_res.push(weekday_hdrs);
    tuple_dates.iter().for_each(|ds| {
        let mut res = String::new();
        if let Some((d, w, date)) = ds.iter().take(1).next() {
            let d = if today == *date {
                let style = Style::new().reverse();
                style.paint(d.to_string()).to_string()
            } else {
                let style = Style::new();
                style.paint(d.to_string()).to_string()
            };
            match w {
                Weekday::Sun => res.push_str(&format!("{:>2}", d)),
                Weekday::Mon => res.push_str(&format!("{:>5}", d)),
                Weekday::Tue => res.push_str(&format!("{:>8}", d)),
                Weekday::Wed => res.push_str(&format!("{:>11}", d)),
                Weekday::Thu => res.push_str(&format!("{:>14}", d)),
                Weekday::Fri => res.push_str(&format!("{:>17}", d)),
                Weekday::Sat => res.push_str(&format!("{:>20}", d)),
            }
        }
        for (d, _, date) in ds.iter().skip(1) {
            if today == *date {
                let style = Style::new().reverse();
                let str = style.paint(format!("{:>2}", d)).to_string();
                res.push_str(&format!(" {}", str));
            } else {
                let style = Style::new();
                let str = style.paint(d.to_string()).to_string();
                res.push_str(&format!("{:>3}", str));
            };
        }

        let mut res = format!("{:22}", res);
        if res.len() > 22 {
            res.push_str("  ");
        }

        print_res.push(res);
    });
    let end_spaces = format!("{:22}", " ");
    if last_day.weekday() != Weekday::Sun {
        print_res.push(end_spaces)
    };
    print_res
}

fn last_day_in_month(year: i32, month: u32) -> NaiveDate {
    let next =
        NaiveDate::from_ymd_opt(year, month + 1, 1).unwrap_or(NaiveDate::from_ymd(year + 1, 1, 1));
    next - Duration::days(1)
}

#[cfg(test)]
mod test {
    use super::{format_month, last_day_in_month};
    use chrono::NaiveDate;

    #[test]
    fn test_format_month() {
        let today = NaiveDate::from_ymd(0, 1, 1);
        let leap_february = vec![
            "   February 2020      ",
            "Su Mo Tu We Th Fr Sa  ",
            "                   1  ",
            " 2  3  4  5  6  7  8  ",
            " 9 10 11 12 13 14 15  ",
            "16 17 18 19 20 21 22  ",
            "23 24 25 26 27 28 29  ",
            "                      ",
        ];
        assert_eq!(format_month(2020, 2, true, today), leap_february);

        let may = vec![
            "        May           ",
            "Su Mo Tu We Th Fr Sa  ",
            "                1  2  ",
            " 3  4  5  6  7  8  9  ",
            "10 11 12 13 14 15 16  ",
            "17 18 19 20 21 22 23  ",
            "24 25 26 27 28 29 30  ",
            "31                    ",
        ];
        assert_eq!(format_month(2020, 5, false, today), may);

        let april_hl = vec![
            "     April 2021       ",
            "Su Mo Tu We Th Fr Sa  ",
            "             1  2  3  ",
            " 4  5  6 \u{1b}[7m 7\u{1b}[0m  8  9 10  ",
            "11 12 13 14 15 16 17  ",
            "18 19 20 21 22 23 24  ",
            "25 26 27 28 29 30     ",
            "                      ",
        ];
        let today = NaiveDate::from_ymd(2021, 4, 7);
        assert_eq!(format_month(2021, 4, true, today), april_hl);
    }
    #[test]
    fn test_last_day_in_month() {
        assert_eq!(last_day_in_month(2020, 1), NaiveDate::from_ymd(2020, 1, 31));
        assert_eq!(last_day_in_month(2020, 2), NaiveDate::from_ymd(2020, 2, 29));
        assert_eq!(last_day_in_month(2020, 4), NaiveDate::from_ymd(2020, 4, 30));
    }
}
