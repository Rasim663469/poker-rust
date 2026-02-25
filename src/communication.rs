// Dans src/communication.rs
use serde::{Serialize, Deserialize};
use crate::carte::Carte;

#[derive(Serialize, Deserialize, Debug)]
pub enum ActionJoueur {
    Fold,
    Check,
    Call,
    Raise(u32),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum MessageClient {
    Connexion { pseudo: String },
    Action(ActionJoueur), 
    Info{info:String},
}

#[derive(Serialize, Deserialize, Debug)]
pub enum MessageServeur {
    Bienvenue { message: String },
    MesCartes { cartes: Vec<Carte> },
    MajTable { pot: u32, cartes_communes: Vec<Carte> },
    DemanderAction { to_call: u32, peut_relancer: bool,jetons_restants: u32, }, 
    AnnonceAction { nom: String, action: String },
}