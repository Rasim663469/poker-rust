use rand::seq::SliceRandom;
use rand::thread_rng;
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Couleur {
    Coeur,
    Carreau,
    Trefle,
    Pique,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Valeur {
    Deux,
    Trois,
    Quatre,
    Cinq,
    Six,
    Sept,
    Huit,
    Neuf,
    Dix,
    Valet,
    Dame,
    Roi,
    As,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Carte {
    pub valeur: Valeur,
    pub couleur: Couleur,
}

pub struct Paquet {
    pub cartes: Vec<Carte>,
}

impl Paquet {
    pub fn nouveau() -> Self {
        let mut cartes = Vec::with_capacity(52);
        for &couleur in &[Couleur::Coeur, Couleur::Carreau, Couleur::Trefle, Couleur::Pique] {
            for &valeur in &[
                Valeur::Deux,
                Valeur::Trois,
                Valeur::Quatre,
                Valeur::Cinq,
                Valeur::Six,
                Valeur::Sept,
                Valeur::Huit,
                Valeur::Neuf,
                Valeur::Dix,
                Valeur::Valet,
                Valeur::Dame,
                Valeur::Roi,
                Valeur::As,
            ] {
                cartes.push(Carte { valeur, couleur });
            }
        }
        Paquet { cartes }
    }

    pub fn melanger(&mut self) {
        let mut rng = thread_rng();
        self.cartes.shuffle(&mut rng);
    }

    pub fn tirer_carte(&mut self) -> Option<Carte> {
        self.cartes.pop()
    }
}

impl Valeur {
    pub fn en_u8(self) -> u8 {
        match self {
            Valeur::Deux => 2,
            Valeur::Trois => 3,
            Valeur::Quatre => 4,
            Valeur::Cinq => 5,
            Valeur::Six => 6,
            Valeur::Sept => 7,
            Valeur::Huit => 8,
            Valeur::Neuf => 9,
            Valeur::Dix => 10,
            Valeur::Valet => 11,
            Valeur::Dame => 12,
            Valeur::Roi => 13,
            Valeur::As => 14,
        }
    }
}

impl fmt::Display for Couleur {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let symbole = match self {
            Couleur::Coeur => "C",
            Couleur::Carreau => "D",
            Couleur::Trefle => "T",
            Couleur::Pique => "P",
        };
        write!(f, "{}", symbole)
    }
}

impl fmt::Display for Valeur {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let texte = match self {
            Valeur::Deux => "2",
            Valeur::Trois => "3",
            Valeur::Quatre => "4",
            Valeur::Cinq => "5",
            Valeur::Six => "6",
            Valeur::Sept => "7",
            Valeur::Huit => "8",
            Valeur::Neuf => "9",
            Valeur::Dix => "10",
            Valeur::Valet => "V",
            Valeur::Dame => "D",
            Valeur::Roi => "R",
            Valeur::As => "A",
        };
        write!(f, "{}", texte)
    }
}

impl fmt::Display for Carte {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.valeur, self.couleur)
    }
}
