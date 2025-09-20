use std::env;
use std::fs;
use std::io::{self, BufRead};
use std::path::Path;

struct Flags {
    ignore_case: bool,
    count: bool,
    line_number: bool,
    color: bool,
    invert_match: bool,
    recursive: bool,
}
fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() > 1 && args[1] == "--help" {
        let help = include_str!("../help.txt");
        println!("{}", help);
        return;
    }

    let pattern_and_file_args: Vec<&String> = args
        .iter()
        .filter(|arg: &&String| !arg.starts_with("-"))
        .collect::<Vec<_>>();
    if pattern_and_file_args.len() < 2 {
        eprintln!("Usage: rgrep [OPTIONS]... PATTERNS [FILE]...");
        eprintln!("Try 'rgrep help' for more information.");
    }
    let flags = Flags {
        ignore_case: args
            .iter()
            .any(|arg: &String| arg.starts_with("-i") || arg.starts_with("--ignore-case")),
        count: args
            .iter()
            .any(|arg| arg.starts_with("-c") || arg.starts_with("--count")),
        line_number: args
            .iter()
            .any(|arg| arg.starts_with("-n") || arg.starts_with("--line-number")),
        color: args
            .iter()
            .any(|arg| arg.starts_with("-C") || arg.starts_with("--color")),
        invert_match: args
            .iter()
            .any(|arg| arg.starts_with("-v") || arg.starts_with("--invert-match")),
        recursive: args.iter().any(|arg| {
            arg.starts_with("-R") || arg.starts_with("-r") || arg.starts_with("--recursive")
        }),
    };
    let pattern: &&String = &pattern_and_file_args[0];
    let file_paths: &[&String] = &pattern_and_file_args[1..];
    let mut total_match_count = 0;
    for path in file_paths {
        match lazy_load(path, pattern, &flags) {
            Ok(c) => {
                if flags.count {
                    total_match_count += c;
                }
            }
            Err(e) => {
                eprintln!("Error processing file {}: {}", path, e);
            }
        }
    }
    if flags.count {
        println!("Total Match Count :{}", total_match_count);
    }
}

fn is_match(line: &str, pattern: &str, flags: &Flags) -> bool {
    let (line_cmp, pat_cmp) = if flags.ignore_case {
        (line.to_lowercase(), pattern.to_lowercase())
    } else {
        (line.to_string(), pattern.to_string())
    };
    let found = line_cmp.contains(&pat_cmp);
    if flags.invert_match { !found } else { found }
}

fn highlight(line: &str, pattern: &str, flags: &Flags) -> String {
    if flags.color {
        line.replace(pattern, &format!("\x1b[31m{}\x1b[0m", pattern))
    } else {
        line.to_string()
    }
}

fn lazy_load(path: &str, pattern: &str, flags: &Flags) -> io::Result<usize> {
    let mut match_count = 0;
    let path_obj = Path::new(path);

    if flags.recursive && path_obj.is_dir() {
        // Iterate over directory entries
        let entries = fs::read_dir(path_obj)?;
        for entry in entries {
            let entry = entry?;
            let entry_path = entry.path();
            if let Some(entry_str) = entry_path.to_str() {
                match_count += lazy_load(entry_str, pattern, flags)?;
            }
        }
        return Ok(match_count); // done processing directory
    }

    let file = fs::File::open(path_obj)?;
    let reader = io::BufReader::new(file);

    for (idx, line) in reader.lines().enumerate() {
        let line = line?;
        if is_match(&line, pattern, flags) {
            match_count += 1;
            if !flags.count {
                let output_line = highlight(&line, pattern, flags);

                if flags.line_number {
                    println!("{}:{}: {}", path, idx + 1, output_line);
                } else {
                    println!("{}: {}", path, output_line);
                }
            }
        }
    }

    if flags.count {
        println!("{}: match count {}", path, match_count);
        Ok(match_count)
    } else {
        Ok(0)
    }
}

// fn easy_load(path: &str, pattern: &str, flags: &Flags) {
//     match std::fs::read_to_string(path) {
//         Ok(c) => {
//             let mut match_count = 0;
//             for (idx, line) in c.lines().enumerate() {
//                 if is_match(&line, pattern, flags) {
//                     if flags.count {
//                         match_count += 1;
//                         continue;
//                     }

//                     let output_line = highlight(&line, pattern, flags);

//                     if flags.line_number {
//                         println!("{}:{}: {}", path, idx + 1, output_line);
//                     } else {
//                         println!("{}: {}", path, output_line);
//                     }
//                 }
//             }

//             if flags.count {
//                 println!("{}: match count {}", path, match_count);
//             }
//         }
//         Err(e) => {
//             eprintln!("Error reading file {}: {}", path, e);
//         }
//     }
// }
