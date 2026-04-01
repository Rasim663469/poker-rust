use crate::core::cards::Carte;

pub struct Joueur {
    // Ce type reste volontairement simple :
    // il représente juste un joueur de jeu de cartes avec sa main, ses jetons et son état sur le tour.
    pub nom: String,
    pub main: Vec<Carte>,
    pub jetons: u32,
    pub couche: bool,
    pub mise_tour: u32,
}

impl Joueur {
    pub fn nouveau(nom: String, jetons: u32) -> Self {
        Self {
            nom,
            main: Vec::new(),
            jetons,
            couche: false,
            mise_tour: 0,
        }
    }

    pub fn afficher_main(&self, montrer_cartes: bool) {
        // Cette méthode sert surtout pour le mode terminal.
        // En interface graphique, on affiche les cartes autrement.
        println!("{} ({} jetons)", self.nom, self.jetons);
        if !montrer_cartes {
            println!("  [cartes cachees]");
            return;
        }

        for (index, carte) in self.main.iter().enumerate() {
            println!("  {}. {}", index + 1, carte);
        }
    }
}
