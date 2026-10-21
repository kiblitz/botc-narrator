//! Stands in for `codex exec` in tests: reads the prompt from stdin and prints
//! its arguments (default "0"), or with `--fail MESSAGE` fails with MESSAGE.

use std::io::Read;

fn main() {
    let mut prompt = String::new();
    std::io::stdin()
        .read_to_string(&mut prompt)
        .expect("a prompt on stdin");
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("--fail") {
        eprintln!("{}", args[1..].join(" "));
        std::process::exit(1);
    }
    if args.is_empty() {
        println!("0");
    } else {
        println!("{}", args.join(" "));
    }
}
