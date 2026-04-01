use crate::core::cards::Carte;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ActionJoueur {
    // Ces actions sont les commandes "métier" envoyées par le client pendant une partie.
    Fold,
    Check,
    Call,
    Raise(u32),
    ConfigurerPartie { nb_joueurs: u32, jetons: u32 },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum MessageClient {
    // Ce sont les messages qui partent du client vers le serveur :
    // soit pour l'auth, soit pour la configuration, soit pour jouer.
    Connexion { pseudo: String },
    Session { db_id: i32, pseudo: String },
    Login { pseudo: String, mot_de_passe: String },
    Inscription { pseudo: String, mot_de_passe: String },
    Action(ActionJoueur),
}


#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum MessageServeur {
    // Même principe côté serveur :
    // on préfère un protocole explicite plutôt que des chaînes de caractères ambiguës.
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
    Erreur { message: String },
    AuthOk { jetons: u32 },
    AuthEchec { raison: String },
}
