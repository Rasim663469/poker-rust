mod carte;
mod interface;
mod joueur;
mod partie;
mod utils;

fn main() {
    let mode_cli = std::env::args().any(|arg| arg == "--cli");

    if mode_cli {
        interface::terminal::lancer_poker_cli();
        return;
    }

    if let Err(err) = interface::gui::lancer_gui() {
        eprintln!("Impossible de lancer la GUI: {}", err);
        eprintln!("Tu peux lancer le jeu en terminal avec: cargo run -- --cli");
    }
}
