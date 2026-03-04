use crate::core::cards::{Carte, Paquet, Valeur};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EtatBlackjack {
    EnAttenteMise,
    TourJoueur,
    TourCroupier,
    Termine,
}

pub struct JoueurBlackjack {
    pub nom: String,
    pub est_bot: bool,
    pub main: Vec<Carte>,
    pub jetons: u32,
    pub mise: u32,
    pub stand: bool,
    pub bust: bool,
    pub blackjack: bool,
}

impl JoueurBlackjack {
    fn nouveau(nom: String, est_bot: bool, jetons: u32) -> Self {
        Self {
            nom,
            est_bot,
            main: Vec::new(),
            jetons,
            mise: 0,
            stand: false,
            bust: false,
            blackjack: false,
        }
    }

    pub fn actif(&self) -> bool {
        self.mise > 0
    }

    fn en_jeu(&self) -> bool {
        self.actif() && !self.stand && !self.bust && !self.blackjack
    }
}

pub struct JeuBlackjack {
    paquet: Paquet,
    pub joueurs: Vec<JoueurBlackjack>,
    pub main_croupier: Vec<Carte>,
    pub etat: EtatBlackjack,
    pub message: String,
    pub joueur_courant: Option<usize>,
    pub mise_reference: u32,
}

impl JeuBlackjack {
    pub fn nouveau(nb_joueurs: usize, jetons_depart: u32) -> Self {
        let mut paquet = Paquet::nouveau();
        paquet.melanger();

        let nb = nb_joueurs.clamp(2, 6);
        let mut joueurs = Vec::with_capacity(nb);
        joueurs.push(JoueurBlackjack::nouveau("Toi".to_string(), false, jetons_depart));
        for i in 1..nb {
            joueurs.push(JoueurBlackjack::nouveau(
                format!("Bot{}", i),
                true,
                jetons_depart,
            ));
        }

        Self {
            paquet,
            joueurs,
            main_croupier: Vec::new(),
            etat: EtatBlackjack::EnAttenteMise,
            message: "Place une mise pour commencer.".to_string(),
            joueur_courant: None,
            mise_reference: 0,
        }
    }

    pub fn jetons_humain(&self) -> u32 {
        self.joueurs.first().map(|j| j.jetons).unwrap_or(0)
    }

    pub fn est_tour_humain(&self) -> bool {
        self.etat == EtatBlackjack::TourJoueur && self.joueur_courant == Some(0)
    }

    pub fn commencer_manche(&mut self, mise_humain: u32) -> Result<(), String> {
        if self.jetons_humain() == 0 {
            return Err("Tu n'as plus de jetons.".to_string());
        }
        if mise_humain == 0 || mise_humain > self.jetons_humain() {
            return Err("Mise invalide.".to_string());
        }

        if self.paquet.cartes.len() < 30 {
            self.paquet = Paquet::nouveau();
            self.paquet.melanger();
        }

        self.main_croupier.clear();
        self.mise_reference = mise_humain;
        self.joueur_courant = None;

        for (idx, j) in self.joueurs.iter_mut().enumerate() {
            j.main.clear();
            j.mise = 0;
            j.stand = false;
            j.bust = false;
            j.blackjack = false;

            if j.jetons == 0 {
                continue;
            }
            let mise = if idx == 0 {
                mise_humain
            } else {
                mise_humain.min(j.jetons)
            };
            j.jetons -= mise;
            j.mise = mise;
        }

        if self.joueurs.iter().all(|j| !j.actif()) {
            return Err("Aucun joueur actif pour cette manche.".to_string());
        }

        for _ in 0..2 {
            for i in 0..self.joueurs.len() {
                if self.joueurs[i].actif() {
                    self.tirer_joueur(i);
                }
            }
            self.tirer_croupier();
        }

        for j in &mut self.joueurs {
            if j.actif() && est_blackjack(&j.main) {
                j.blackjack = true;
            }
        }

        self.etat = EtatBlackjack::TourJoueur;
        self.joueur_courant = self.prochain_joueur_a_jouer(0);
        self.message = "Manche distribuée. Tour des joueurs.".to_string();

        if self.joueur_courant.is_none() {
            self.passer_au_croupier();
        }
        Ok(())
    }

    pub fn joueur_hit(&mut self) {
        if !self.est_tour_humain() {
            return;
        }
        let idx = 0;
        self.tirer_joueur(idx);
        let score = self.score_joueur(idx);
        if score > 21 {
            self.joueurs[idx].bust = true;
            self.message = "Tu bust (plus de 21).".to_string();
            self.avancer_tour_joueur();
        } else {
            self.message = format!("Tu tires une carte. Score: {}", score);
            // En blackjack, le joueur peut continuer a tirer tant qu'il ne bust pas.
        }
    }

    pub fn joueur_stand(&mut self) {
        if !self.est_tour_humain() {
            return;
        }
        self.joueurs[0].stand = true;
        self.message = "Tu restes.".to_string();
        self.avancer_tour_joueur();
    }

    pub fn avancer_automatique(&mut self) {
        while self.etat == EtatBlackjack::TourJoueur {
            let Some(idx) = self.joueur_courant else {
                self.passer_au_croupier();
                break;
            };
            if !self.joueurs[idx].est_bot {
                break;
            }
            let tour_fini = self.jouer_bot(idx);
            if tour_fini {
                self.avancer_tour_joueur();
            }
        }
    }

    pub fn score_joueur(&self, idx: usize) -> u8 {
        self.joueurs
            .get(idx)
            .map(|j| valeur_main(&j.main).0)
            .unwrap_or(0)
    }

    pub fn score_croupier(&self) -> u8 {
        valeur_main(&self.main_croupier).0
    }

    pub fn score_croupier_visible(&self) -> u8 {
        if self.main_croupier.is_empty() {
            0
        } else if self.croupier_cachee() && self.main_croupier.len() >= 2 {
            valeur_main(&self.main_croupier[1..]).0
        } else {
            self.score_croupier()
        }
    }

    pub fn croupier_cachee(&self) -> bool {
        self.etat == EtatBlackjack::TourJoueur
    }

    fn tirer_joueur(&mut self, idx: usize) {
        if let Some(c) = self.paquet.tirer_carte() {
            self.joueurs[idx].main.push(c);
        }
    }

    fn tirer_croupier(&mut self) {
        if let Some(c) = self.paquet.tirer_carte() {
            self.main_croupier.push(c);
        }
    }

    fn prochain_joueur_a_jouer(&self, start_inclus: usize) -> Option<usize> {
        (start_inclus..self.joueurs.len()).find(|&i| self.joueurs[i].en_jeu())
    }

    fn avancer_tour_joueur(&mut self) {
        if self.etat != EtatBlackjack::TourJoueur {
            return;
        }
        let next_start = self.joueur_courant.map(|i| i + 1).unwrap_or(0);
        self.joueur_courant = self.prochain_joueur_a_jouer(next_start);
        if self.joueur_courant.is_none() {
            self.passer_au_croupier();
        }
    }

    fn jouer_bot(&mut self, idx: usize) -> bool {
        let score = self.score_joueur(idx);
        if score < 16 {
            self.tirer_joueur(idx);
            let new_score = self.score_joueur(idx);
            if new_score > 21 {
                self.joueurs[idx].bust = true;
                self.message = format!("{} bust ({}).", self.joueurs[idx].nom, new_score);
                return true;
            } else {
                self.message = format!("{} hit ({}).", self.joueurs[idx].nom, new_score);
                return false;
            }
        } else {
            self.joueurs[idx].stand = true;
            self.message = format!("{} stand ({}).", self.joueurs[idx].nom, score);
            return true;
        }
    }

    fn passer_au_croupier(&mut self) {
        self.etat = EtatBlackjack::TourCroupier;
        self.jouer_croupier();
        self.resoudre_manche();
    }

    fn jouer_croupier(&mut self) {
        while self.score_croupier() < 17 {
            self.tirer_croupier();
        }
    }

    fn resoudre_manche(&mut self) {
        let score_c = self.score_croupier();
        let croupier_bj = est_blackjack(&self.main_croupier);
        let croupier_bust = score_c > 21;

        let mut recap = Vec::new();
        for j in &mut self.joueurs {
            if !j.actif() {
                continue;
            }
            let score_j = valeur_main(&j.main).0;
            let mut gain = 0u32;

            if j.blackjack && croupier_bj {
                gain = j.mise;
                recap.push(format!("{}: push (BJ)", j.nom));
            } else if j.blackjack {
                gain = j.mise + (j.mise * 3) / 2;
                recap.push(format!("{}: blackjack", j.nom));
            } else if j.bust {
                recap.push(format!("{}: bust", j.nom));
            } else if croupier_bust {
                gain = j.mise * 2;
                recap.push(format!("{}: gagne (croupier bust)", j.nom));
            } else if score_j > score_c {
                gain = j.mise * 2;
                recap.push(format!("{}: gagne", j.nom));
            } else if score_j == score_c {
                gain = j.mise;
                recap.push(format!("{}: push", j.nom));
            } else {
                recap.push(format!("{}: perd", j.nom));
            }

            j.jetons += gain;
        }

        self.message = format!("Résultat: {}", recap.join(" | "));
        self.etat = EtatBlackjack::Termine;
        self.joueur_courant = None;
    }
}

pub fn valeur_main(main: &[Carte]) -> (u8, bool) {
    let mut total = 0u8;
    let mut as_count = 0u8;

    for c in main {
        match c.valeur {
            Valeur::Deux => total += 2,
            Valeur::Trois => total += 3,
            Valeur::Quatre => total += 4,
            Valeur::Cinq => total += 5,
            Valeur::Six => total += 6,
            Valeur::Sept => total += 7,
            Valeur::Huit => total += 8,
            Valeur::Neuf => total += 9,
            Valeur::Dix | Valeur::Valet | Valeur::Dame | Valeur::Roi => total += 10,
            Valeur::As => {
                total += 11;
                as_count += 1;
            }
        }
    }

    while total > 21 && as_count > 0 {
        total -= 10;
        as_count -= 1;
    }
    (total, as_count > 0)
}

pub fn est_blackjack(main: &[Carte]) -> bool {
    main.len() == 2 && valeur_main(main).0 == 21
}
