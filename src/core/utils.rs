use std::io::{self, Write};

pub fn demander(message: &str) -> io::Result<String> {
    print!("{}", message);
    io::stdout().flush()?;

    let mut entree = String::new();
    io::stdin().read_line(&mut entree)?;
    Ok(entree.trim().to_string())
}

pub fn demander_u32(message: &str, min: u32, max: u32) -> u32 {
    loop {
        match demander(message) {
            Ok(entree) => match entree.parse::<u32>() {
                Ok(v) if v >= min && v <= max => return v,
                _ => println!("Entree invalide. Valeur attendue: {}..={}", min, max),
            },
            Err(_) => println!("Erreur de lecture, reessaie."),
        }
    }
}
