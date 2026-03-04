use crate::core::cards::Carte;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ActionJoueur {
    Fold,
    Check,
    Call,
    Raise(u32),
    ConfigurerPartie { nb_joueurs: u32, jetons: u32 },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum MessageClient {
    Connexion { pseudo: String },
    Action(ActionJoueur),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum MessageServeur {
    Bienvenue { message: String },
    MesCartes { cartes: Vec<Carte> },
    MajTable { pot: u32, cartes_communes: Vec<Carte> },
    DemanderAction {
        to_call: u32,
        peut_relancer: bool,
        jetons_restants: u32,
    },
    AnnonceAction { nom: String, action: String },
    DemanderConfiguration,
}
