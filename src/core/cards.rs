use rand::seq::SliceRandom;
use rand::thread_rng;
use serde::Deserialize;
use std::collections::HashMap;
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
    deck_api_id: Option<String>,
    pile_api_nom: String,
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
        let deck_api_id = api_creer_deck_id();
        Paquet {
            cartes,
            deck_api_id,
            pile_api_nom: "rust_game".to_string(),
        }
    }

    pub fn melanger(&mut self) {
        if let Some(deck_id) = &self.deck_api_id {
            if !api_melanger(deck_id) {
                self.deck_api_id = None;
            }
        }
        let mut rng = thread_rng();
        self.cartes.shuffle(&mut rng);
    }

    pub fn tirer_carte(&mut self) -> Option<Carte> {
        if let Some(deck_id) = self.deck_api_id.as_deref() {
            if let Some(carte) = api_tirer_carte(deck_id) {
                let _ = api_ajouter_a_pile(deck_id, &self.pile_api_nom, &carte.code_api());
                self.cartes.retain(|c| *c != carte);
                return Some(carte);
            }
            self.deck_api_id = None;
        }
        self.cartes.pop()
    }

    #[allow(dead_code)]
    pub fn lister_cartes_tirees_api(&self) -> Option<Vec<Carte>> {
        let deck_id = self.deck_api_id.as_deref()?;
        api_lister_pile(deck_id, &self.pile_api_nom)
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

impl Carte {
    pub fn code_api(&self) -> String {
        format!("{}{}", valeur_code_api(self.valeur), couleur_code_api(self.couleur))
    }

    pub fn image_url_api(&self) -> String {
        format!("https://deckofcardsapi.com/static/img/{}.png", self.code_api())
    }
}

#[derive(Deserialize)]
struct ApiDeckResponse {
    success: bool,
    deck_id: String,
}

#[derive(Deserialize)]
struct ApiDrawResponse {
    success: bool,
    cards: Vec<ApiCard>,
}

#[derive(Deserialize)]
struct ApiCard {
    value: String,
    suit: String,
    #[allow(dead_code)]
    image: String,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct ApiPileListResponse {
    success: bool,
    piles: HashMap<String, ApiPileData>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct ApiPileData {
    #[allow(dead_code)]
    remaining: u32,
    cards: Vec<ApiCard>,
}

#[derive(Deserialize)]
struct ApiSuccessOnly {
    success: bool,
}

fn api_creer_deck_id() -> Option<String> {
    let url = "https://deckofcardsapi.com/api/deck/new/shuffle/?deck_count=1";
    let resp = reqwest::blocking::get(url).ok()?;
    let payload: ApiDeckResponse = resp.json().ok()?;
    if payload.success {
        Some(payload.deck_id)
    } else {
        None
    }
}

fn api_melanger(deck_id: &str) -> bool {
    let url = format!("https://deckofcardsapi.com/api/deck/{}/shuffle/", deck_id);
    let resp = match reqwest::blocking::get(url) {
        Ok(r) => r,
        Err(_) => return false,
    };
    match resp.json::<ApiDeckResponse>() {
        Ok(p) => p.success,
        Err(_) => false,
    }
}

fn api_tirer_carte(deck_id: &str) -> Option<Carte> {
    let url = format!("https://deckofcardsapi.com/api/deck/{}/draw/?count=1", deck_id);
    let resp = reqwest::blocking::get(url).ok()?;
    let payload: ApiDrawResponse = resp.json().ok()?;
    if !payload.success {
        return None;
    }
    let card = payload.cards.first()?;
    let valeur = valeur_depuis_api(&card.value)?;
    let couleur = couleur_depuis_api(&card.suit)?;
    Some(Carte { valeur, couleur })
}

fn api_ajouter_a_pile(deck_id: &str, pile_nom: &str, code: &str) -> bool {
    let url = format!(
        "https://deckofcardsapi.com/api/deck/{}/pile/{}/add/?cards={}",
        deck_id, pile_nom, code
    );
    let resp = match reqwest::blocking::get(url) {
        Ok(r) => r,
        Err(_) => return false,
    };
    match resp.json::<ApiSuccessOnly>() {
        Ok(p) => p.success,
        Err(_) => false,
    }
}

#[allow(dead_code)]
fn api_lister_pile(deck_id: &str, pile_nom: &str) -> Option<Vec<Carte>> {
    let url = format!(
        "https://deckofcardsapi.com/api/deck/{}/pile/{}/list/",
        deck_id, pile_nom
    );
    let resp = reqwest::blocking::get(url).ok()?;
    let payload: ApiPileListResponse = resp.json().ok()?;
    if !payload.success {
        return None;
    }
    let pile = payload.piles.get(pile_nom)?;
    let mut cartes = Vec::with_capacity(pile.cards.len());
    for c in &pile.cards {
        let valeur = valeur_depuis_api(&c.value)?;
        let couleur = couleur_depuis_api(&c.suit)?;
        cartes.push(Carte { valeur, couleur });
    }
    Some(cartes)
}

fn valeur_depuis_api(v: &str) -> Option<Valeur> {
    match v {
        "2" => Some(Valeur::Deux),
        "3" => Some(Valeur::Trois),
        "4" => Some(Valeur::Quatre),
        "5" => Some(Valeur::Cinq),
        "6" => Some(Valeur::Six),
        "7" => Some(Valeur::Sept),
        "8" => Some(Valeur::Huit),
        "9" => Some(Valeur::Neuf),
        "10" => Some(Valeur::Dix),
        "JACK" => Some(Valeur::Valet),
        "QUEEN" => Some(Valeur::Dame),
        "KING" => Some(Valeur::Roi),
        "ACE" => Some(Valeur::As),
        _ => None,
    }
}

fn couleur_depuis_api(s: &str) -> Option<Couleur> {
    match s {
        "HEARTS" => Some(Couleur::Coeur),
        "DIAMONDS" => Some(Couleur::Carreau),
        "CLUBS" => Some(Couleur::Trefle),
        "SPADES" => Some(Couleur::Pique),
        _ => None,
    }
}

fn valeur_code_api(v: Valeur) -> &'static str {
    match v {
        Valeur::Deux => "2",
        Valeur::Trois => "3",
        Valeur::Quatre => "4",
        Valeur::Cinq => "5",
        Valeur::Six => "6",
        Valeur::Sept => "7",
        Valeur::Huit => "8",
        Valeur::Neuf => "9",
        Valeur::Dix => "0",
        Valeur::Valet => "J",
        Valeur::Dame => "Q",
        Valeur::Roi => "K",
        Valeur::As => "A",
    }
}

fn couleur_code_api(c: Couleur) -> &'static str {
    match c {
        Couleur::Coeur => "H",
        Couleur::Carreau => "D",
        Couleur::Trefle => "C",
        Couleur::Pique => "S",
    }
}
