use std::env;

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
    let flags = args
        .iter()
        .filter(|arg| arg.starts_with("-"))
        .collect::<Vec<_>>();

    if pattern_and_file_args.len() < 2 {
        eprintln!("Usage: rgrep [OPTIONS]... PATTERNS [FILE]...");
        eprintln!("Try 'rgrep help' for more information.");
    }

    let ignore_case = flags
        .iter()
        .any(|arg| arg.starts_with("-i") || arg.starts_with("--ignore-case"));
    let count = flags
        .iter()
        .any(|arg| arg.starts_with("-c") || arg.starts_with("--count"));
    let line_number = flags
        .iter()
        .any(|arg| arg.starts_with("-n") || arg.starts_with("--line-number"));
    let color = flags
        .iter()
        .any(|arg| arg.starts_with("-C") || arg.starts_with("--color"));
    let pattern = &pattern_and_file_args[0];
    let file_paths = &pattern_and_file_args[1..];
    let mut match_count = 0;
    for path in file_paths {
        let contents = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Error reading file {}: {}", path, e);
                continue;
            }
        };
        for line in contents.lines() {
            if (ignore_case && line.to_lowercase().contains(&pattern.to_lowercase()))
                || (!ignore_case && line.contains(*pattern))
            {
                let line_index: usize = match contents.lines().position(|l| l == line) {
                    Some(idx) if idx > 0 => idx + 1,
                    _ => 0,
                };
                if count {
                    match_count += 1;
                }
                if line_number && !count {
                    if color {
                        let highlighted_line: String =
                            line.replace(*pattern, &format!("\x1b[31m{}\x1b[0m", pattern));
                        println!("{}:{}: {}", path, line_index, highlighted_line);
                    } else {
                        println!("{}:{}: {}", path, line_index, line);
                    }
                } else if !count {
                    if color {
                        let highlighted_line: String =
                            line.replace(*pattern, &format!("\x1b[31m{}\x1b[0m", pattern));
                        println!("{}:{}: {}", path, line_index, highlighted_line);
                    } else {
                        println!("{}:{}: {}", path, line_index, line);
                    }
                }
            }
        }
        if count {
            println!("{}: match count {}", path, match_count);
            match_count = 0; // Reset for next file
        }
    }
}
