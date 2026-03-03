use std::io::{self, Write};

pub fn demander(message: &str) -> io::Result<String> {
    print!("{}", message);
    io::stdout().flush()?;
    let mut entree = String::new();
    io::stdin().read_line(&mut entree)?;
    Ok(entree.trim().to_string())
}
