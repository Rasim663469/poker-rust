mod core;
mod games;
mod interface;

fn main() {
    let mode_cli = std::env::args().any(|arg| arg == "--cli");

    if mode_cli {
        interface::terminal::lancer_casino_cli();
        return;
    }

    if let Err(err) = interface::gui::lancer_gui() {
        eprintln!("Impossible de lancer la GUI: {}", err);
        eprintln!("Tu peux lancer le jeu en terminal avec: cargo run -- --cli");
    }
}
