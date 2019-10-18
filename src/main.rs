use std::io;
use std::io::prelude::*;
use std::path::{Path, PathBuf};

use clap::{App, Arg};
use colored::Colorize;
use regex::{Regex, RegexBuilder};

#[derive(Clone, Copy, Debug)]
struct GrepLike<'a> {
    prefix: Option<&'a str>,
    filepath: &'a str,
    row: Option<&'a str>,
    column: Option<&'a str>,
    contents: &'a str,
}

impl<'a> GrepLike<'a> {
    fn write<'s, 'r, 'p>(
        &self,
        mut w: impl Write,
        extra_prefix: Option<&'s str>,
        highlight: Option<&'r Regex>,
        current_dir: &'p Path,
        check_exists: bool,
    ) -> io::Result<()> {
        let filepath: PathBuf = match (self.prefix, extra_prefix) {
            (Some(prefix), Some(extra)) => extra.to_string() + prefix + "/" + self.filepath,
            (Some(prefix), None) => prefix.to_string() + "/" + self.filepath,
            (None, Some(extra)) => extra.to_string() + self.filepath,
            (None, None) => self.filepath.to_string(),
        }
        .into();
        let rel_filepath = pathdiff::diff_paths(&filepath, current_dir).unwrap();
        if check_exists {
            if std::fs::File::open(&rel_filepath).is_err() {
                return Ok(());
            }
        }

        write!(
            w,
            "{}:{}:{}: ",
            rel_filepath.to_str().unwrap().yellow(),
            self.row.unwrap_or("0").blue(),
            self.column.unwrap_or("0").green(),
        )?;
        match highlight {
            Some(re) => {
                let mut offset = 0;
                for caps in re.captures_iter(&self.contents) {
                    let g = caps.get(0).unwrap();
                    write!(w, "{}", &self.contents[offset..g.start()])?;
                    write!(w, "{}", g.as_str().red())?;
                    offset = g.end();
                }
                writeln!(w, "{}", &self.contents[offset..])?;
            }
            None => writeln!(w, "{}", self.contents)?,
        };
        Ok(())
    }
}

fn main() {
    let matches = App::new("grep-wrapper")
        .version("1.0")
        .author("Robert Ying <rbtying@aeturnalus.com>")
        .arg(
            Arg::with_name("prefix")
                .short("p")
                .long("prefix")
                .value_name("PREFIX")
                .help("A string to prefix everything with")
                .required(false)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("highlight")
                .short("h")
                .long("highlight")
                .value_name("HIGHLIGHT_REGEX")
                .help("The regex for items to highlight")
                .required(false)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("check_exists")
                .short("c")
                .long("check_exists")
                .help("Include only file paths that exist on disk"),
        )
        .get_matches();
    let extra_prefix = matches.value_of("prefix");
    let check_exists = matches.value_of("check_exists").is_some();
    let highlight_regex = matches.value_of("highlight").map(|h| {
        RegexBuilder::new(&h)
            .case_insensitive(true)
            .build()
            .unwrap()
    });
    let line_regex =
        Regex::new(r#"(?:[^:/]+/?([^:]+):)?([^:]+)(?::(\d+))?(?::(\d+))?:\s*(.*)"#).unwrap();

    let cwd = std::env::current_dir().unwrap();

    for line in std::io::stdin().lock().lines() {
        match line {
            Ok(line) => {
                if let Some(captures) = line_regex.captures(&line) {
                    let s = GrepLike {
                        prefix: captures.get(1).map(|s| s.as_str()),
                        filepath: captures.get(2).unwrap().as_str(),
                        row: captures.get(3).map(|s| s.as_str()),
                        column: captures.get(4).map(|s| s.as_str()),
                        contents: captures.get(5).unwrap().as_str(),
                    };

                    let _ = s.write(
                        &mut std::io::stdout(),
                        extra_prefix,
                        highlight_regex.as_ref(),
                        &cwd,
                        check_exists,
                    );
                } else {
                    println!("{}", line);
                }
            }
            Err(e) => {
                eprintln!("err {:?}", e);
            }
        }
    }
}
