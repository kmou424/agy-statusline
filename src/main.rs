use std::io::{self, Read, Write};

fn main() {
    let mut input = String::new();
    let _ = io::stdin().read_to_string(&mut input);

    let output = agy_statusline::render_statusline(&input);
    if !output.is_empty() {
        let mut stdout = io::stdout().lock();
        let _ = writeln!(stdout, "{output}");
    }
}
