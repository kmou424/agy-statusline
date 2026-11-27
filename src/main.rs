use std::{
    env,
    ffi::OsStr,
    io::{self, Read, Write},
    process,
};

const USAGE: &str = "Usage: agy-statusline [title]";

fn main() {
    let mut args = env::args_os().skip(1);
    let arg = args.next();
    let render: fn(&str) -> String = match (arg.as_deref(), args.next()) {
        (None, None) => agy_statusline::render_statusline,
        (Some(arg), None) if arg == OsStr::new("title") => agy_statusline::render_title,
        (Some(arg), None) if arg == OsStr::new("-h") || arg == OsStr::new("--help") => {
            println!(
                "{USAGE}\n\nCommands:\n  title          Render the terminal title\n\n\
                 Options:\n  -h, --help     Print help\n  -V, --version  Print version"
            );
            return;
        }
        (Some(arg), None) if arg == OsStr::new("-V") || arg == OsStr::new("--version") => {
            println!("agy-statusline {}", env!("CARGO_PKG_VERSION"));
            return;
        }
        _ => {
            eprintln!("error: unexpected arguments\n{USAGE}; see --help");
            process::exit(2);
        }
    };

    let mut input = String::new();
    let _ = io::stdin().read_to_string(&mut input);

    let output = render(&input);
    if !output.is_empty() {
        let mut stdout = io::stdout().lock();
        let _ = writeln!(stdout, "{output}");
    }
}
