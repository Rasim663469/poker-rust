use crate::core::cards::{Carte, Paquet};
use crate::core::player::Joueur;
use crate::core::utils::{demander, demander_u32};
use std::cmp::Ordering;
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
enum RangMain {
    CarteHaute,
    Paire,
    DoublePaire,
    Brelan,
    Suite,
    Couleur,
    Full,
    Carre,
    QuinteFlush,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct MainEvaluee {
    rang: RangMain,
    departage: Vec<u8>,
}

impl Ord for MainEvaluee {
    fn cmp(&self, other: &Self) -> Ordering {
        self.rang
            .cmp(&other.rang)
            .then_with(|| self.departage.cmp(&other.departage))
    }
}

impl PartialOrd for MainEvaluee {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub struct Partie {
    pub joueurs: Vec<Joueur>,
    paquet: Paquet,
    cartes_communes: Vec<Carte>,
    pot: u32,
    dealer_idx: usize,
    small_blind: u32,
    big_blind: u32,
}

impl Partie {
    pub fn nouvelle(
        noms: Vec<String>,
        jetons_depart: u32,
        small_blind: u32,
        big_blind: u32,
    ) -> Self {
        let joueurs = noms
            .into_iter()
            .map(|nom| Joueur::nouveau(nom, jetons_depart))
            .collect();
        Self {
            joueurs,
            paquet: Paquet::nouveau(),
            cartes_communes: Vec::new(),
            pot: 0,
            dealer_idx: 0,
            small_blind,
            big_blind,
        }
    }

    pub fn jouer_session_cli(&mut self) {
        loop {
            if self.nb_joueurs_avec_jetons() < 2 {
                println!("Session terminee: moins de 2 joueurs ont des jetons.");
                return;
            }
            self.jouer_manche_holdem_cli();
            let cont = demander("Nouvelle main ? (o/n): ")
                .ok()
                .map(|s| s.eq_ignore_ascii_case("o"))
                .unwrap_or(false);
            if !cont {
                return;
            }
        }
    }

    pub fn jouer_manche_holdem_cli(&mut self) {
        self.preparer_manche();
        if self.nb_joueurs_avec_jetons() < 2 {
            return;
        }

        let (sb_idx, bb_idx) = self.indices_blinds();
        let sb_posee = self.prelever_blind(sb_idx, self.small_blind);
        let bb_posee = self.prelever_blind(bb_idx, self.big_blind);
        let mut mise_actuelle = sb_posee.max(bb_posee);

        self.distribuer_pocket();
        println!("\n=== Nouvelle main (Texas Hold'em) ===");
        println!("Pot initial: {}", self.pot);
        for j in &self.joueurs {
            if j.jetons > 0 || !j.main.is_empty() {
                j.afficher_main(true);
            }
        }

        let mut start = self.prochain_actif(bb_idx);
        self.tour_mises_cli("Preflop", &mut start, &mut mise_actuelle);
        if self.nb_actifs_non_couches() <= 1 {
            self.payer_dernier_actif();
            self.avancer_donneur();
            return;
        }

        self.reinitialiser_mises();
        mise_actuelle = 0;
        self.bruler();
        self.distribuer_communes(3);
        self.afficher_table("Flop");
        start = self.prochain_actif(self.dealer_idx);
        self.tour_mises_cli("Flop", &mut start, &mut mise_actuelle);
        if self.nb_actifs_non_couches() <= 1 {
            self.payer_dernier_actif();
            self.avancer_donneur();
            return;
        }

        self.reinitialiser_mises();
        mise_actuelle = 0;
        self.bruler();
        self.distribuer_communes(1);
        self.afficher_table("Turn");
        start = self.prochain_actif(self.dealer_idx);
        self.tour_mises_cli("Turn", &mut start, &mut mise_actuelle);
        if self.nb_actifs_non_couches() <= 1 {
            self.payer_dernier_actif();
            self.avancer_donneur();
            return;
        }

        self.reinitialiser_mises();
        mise_actuelle = 0;
        self.bruler();
        self.distribuer_communes(1);
        self.afficher_table("River");
        start = self.prochain_actif(self.dealer_idx);
        self.tour_mises_cli("River", &mut start, &mut mise_actuelle);

        if self.nb_actifs_non_couches() <= 1 {
            self.payer_dernier_actif();
        } else {
            self.showdown();
        }
        self.avancer_donneur();
    }

    fn preparer_manche(&mut self) {
        self.paquet = Paquet::nouveau();
        self.paquet.melanger();
        self.cartes_communes.clear();
        self.pot = 0;
        for j in &mut self.joueurs {
            j.main.clear();
            j.couche = j.jetons == 0;
            j.mise_tour = 0;
        }
    }

    fn distribuer_pocket(&mut self) {
        for _ in 0..2 {
            for i in 0..self.joueurs.len() {
                if self.joueurs[i].jetons > 0 {
                    if let Some(c) = self.paquet.tirer_carte() {
                        self.joueurs[i].main.push(c);
                    }
                }
            }
        }
    }

    fn distribuer_communes(&mut self, n: usize) {
        for _ in 0..n {
            if let Some(c) = self.paquet.tirer_carte() {
                self.cartes_communes.push(c);
            }
        }
    }

    fn bruler(&mut self) {
        let _ = self.paquet.tirer_carte();
    }

    fn afficher_table(&self, rue: &str) {
        let board = self
            .cartes_communes
            .iter()
            .map(|c| c.to_string())
            .collect::<Vec<_>>()
            .join(" ");
        println!("\n=== {} ===", rue);
        println!("Board: {}", board);
        println!("Pot: {}", self.pot);
    }

    fn indices_blinds(&self) -> (usize, usize) {
        let sb = self.prochain_actif(self.dealer_idx);
        let bb = self.prochain_actif(sb);
        (sb, bb)
    }

    fn prochain_actif(&self, from: usize) -> usize {
        let n = self.joueurs.len();
        for step in 1..=n {
            let i = (from + step) % n;
            if self.joueurs[i].jetons > 0 && !self.joueurs[i].couche {
                return i;
            }
        }
        from
    }

    fn prelever_blind(&mut self, idx: usize, montant: u32) -> u32 {
        let paye = self.joueurs[idx].jetons.min(montant);
        self.joueurs[idx].jetons -= paye;
        self.joueurs[idx].mise_tour += paye;
        self.pot += paye;
        paye
    }

    fn tour_mises_cli(&mut self, label: &str, start: &mut usize, mise_actuelle: &mut u32) {
        println!("\n--- Tour de mise: {} ---", label);
        let mut actifs = self.nb_actifs_non_couches();
        if actifs <= 1 {
            return;
        }

        let mut besoin_action: Vec<bool> = self
            .joueurs
            .iter()
            .map(|j| !j.couche && j.jetons > 0)
            .collect();
        let mut idx = *start;

        loop {
            if actifs <= 1 {
                break;
            }
            if !besoin_action.iter().any(|&b| b) {
                break;
            }
            if self.joueurs[idx].couche || self.joueurs[idx].jetons == 0 {
                besoin_action[idx] = false;
                idx = self.prochain_actif(idx);
                continue;
            }
            if !besoin_action[idx] {
                idx = self.prochain_actif(idx);
                continue;
            }

            let a_payer = (*mise_actuelle).saturating_sub(self.joueurs[idx].mise_tour);
            println!(
                "{}: jetons={}, mise ce tour={}, a payer={}",
                self.joueurs[idx].nom,
                self.joueurs[idx].jetons,
                self.joueurs[idx].mise_tour,
                a_payer
            );

            if a_payer == 0 {
                let action = demander("Action [c=check, r=relance, f=fold]: ")
                    .ok()
                    .unwrap_or_default()
                    .to_lowercase();
                match action.as_str() {
                    "f" => {
                        self.joueurs[idx].couche = true;
                        besoin_action[idx] = false;
                        actifs -= 1;
                        println!("{} fold.", self.joueurs[idx].nom);
                    }
                    "r" => {
                        let min = if *mise_actuelle == 0 {
                            self.big_blind
                        } else {
                            *mise_actuelle + self.big_blind
                        };
                        let max = self.joueurs[idx].mise_tour + self.joueurs[idx].jetons;
                        if min > max {
                            println!("Relance impossible.");
                            continue;
                        }
                        let total = demander_u32(
                            &format!(
                                "Montant total de ta mise pour ce tour ({}..={}): ",
                                min, max
                            ),
                            min,
                            max,
                        );
                        let delta = total.saturating_sub(self.joueurs[idx].mise_tour);
                        self.joueurs[idx].jetons -= delta;
                        self.joueurs[idx].mise_tour = total;
                        self.pot += delta;
                        *mise_actuelle = total;
                        for j in 0..self.joueurs.len() {
                            besoin_action[j] =
                                !self.joueurs[j].couche && self.joueurs[j].jetons > 0;
                        }
                        besoin_action[idx] = false;
                        println!("{} relance a {}.", self.joueurs[idx].nom, total);
                    }
                    _ => {
                        besoin_action[idx] = false;
                        println!("{} check.", self.joueurs[idx].nom);
                    }
                }
            } else {
                let action = demander("Action [s=suivre, r=relance, f=fold]: ")
                    .ok()
                    .unwrap_or_default()
                    .to_lowercase();
                match action.as_str() {
                    "f" => {
                        self.joueurs[idx].couche = true;
                        besoin_action[idx] = false;
                        actifs -= 1;
                        println!("{} fold.", self.joueurs[idx].nom);
                    }
                    "r" => {
                        let min = (*mise_actuelle + self.big_blind)
                            .max(self.joueurs[idx].mise_tour + a_payer + 1);
                        let max = self.joueurs[idx].mise_tour + self.joueurs[idx].jetons;
                        if min > max {
                            println!("Relance impossible, tu peux seulement suivre ou fold.");
                            continue;
                        }
                        let total = demander_u32(
                            &format!(
                                "Montant total de ta mise pour ce tour ({}..={}): ",
                                min, max
                            ),
                            min,
                            max,
                        );
                        let delta = total.saturating_sub(self.joueurs[idx].mise_tour);
                        self.joueurs[idx].jetons -= delta;
                        self.joueurs[idx].mise_tour = total;
                        self.pot += delta;
                        *mise_actuelle = total;
                        for j in 0..self.joueurs.len() {
                            besoin_action[j] =
                                !self.joueurs[j].couche && self.joueurs[j].jetons > 0;
                        }
                        besoin_action[idx] = false;
                        println!("{} relance a {}.", self.joueurs[idx].nom, total);
                    }
                    _ => {
                        let paye = self.joueurs[idx].jetons.min(a_payer);
                        self.joueurs[idx].jetons -= paye;
                        self.joueurs[idx].mise_tour += paye;
                        self.pot += paye;
                        besoin_action[idx] = false;
                        println!("{} suit.", self.joueurs[idx].nom);
                    }
                }
            }
            idx = self.prochain_actif(idx);
        }
        *start = idx;
    }

    fn reinitialiser_mises(&mut self) {
        for j in &mut self.joueurs {
            j.mise_tour = 0;
        }
    }

    fn nb_actifs_non_couches(&self) -> usize {
        self.joueurs
            .iter()
            .filter(|j| !j.couche && (j.jetons > 0 || !j.main.is_empty()))
            .count()
    }

    fn nb_joueurs_avec_jetons(&self) -> usize {
        self.joueurs.iter().filter(|j| j.jetons > 0).count()
    }

    fn payer_dernier_actif(&mut self) {
        if let Some((idx, _)) = self
            .joueurs
            .iter()
            .enumerate()
            .find(|(_, j)| !j.couche && (j.jetons > 0 || !j.main.is_empty()))
        {
            self.joueurs[idx].jetons += self.pot;
            println!("{} gagne {} (abandon).", self.joueurs[idx].nom, self.pot);
            self.pot = 0;
        }
    }

    fn showdown(&mut self) {
        let mut meilleurs: Vec<(usize, MainEvaluee)> = Vec::new();
        for (i, j) in self.joueurs.iter().enumerate() {
            if j.couche {
                continue;
            }
            let mut cartes = j.main.clone();
            cartes.extend(self.cartes_communes.iter().copied());
            let main = evaluer_holdem(&cartes);
            meilleurs.push((i, main));
        }
        if meilleurs.is_empty() {
            return;
        }

        meilleurs.sort_by(|a, b| b.1.cmp(&a.1));
        let best = meilleurs[0].1.clone();
        let gagnants: Vec<usize> = meilleurs
            .iter()
            .filter(|(_, m)| *m == best)
            .map(|(i, _)| *i)
            .collect();

        let part = self.pot / gagnants.len() as u32;
        let reste = self.pot % gagnants.len() as u32;
        for (k, idx) in gagnants.iter().enumerate() {
            self.joueurs[*idx].jetons += part + if k == 0 { reste } else { 0 };
        }

        let noms = gagnants
            .iter()
            .map(|i| self.joueurs[*i].nom.clone())
            .collect::<Vec<_>>()
            .join(", ");
        println!("Showdown: {} gagnent le pot de {}.", noms, self.pot);
        self.pot = 0;
    }

    fn avancer_donneur(&mut self) {
        self.dealer_idx = (self.dealer_idx + 1) % self.joueurs.len();
    }
}

pub fn evaluer_holdem_pour_gui(cartes: &[Carte]) -> (u8, Vec<u8>, String) {
    let e = evaluer_holdem(cartes);
    (
        e.rang as u8,
        e.departage.clone(),
        nom_rang(e.rang).to_string(),
    )
}

fn nom_rang(rang: RangMain) -> &'static str {
    match rang {
        RangMain::CarteHaute => "Carte haute",
        RangMain::Paire => "Paire",
        RangMain::DoublePaire => "Double paire",
        RangMain::Brelan => "Brelan",
        RangMain::Suite => "Suite",
        RangMain::Couleur => "Couleur",
        RangMain::Full => "Full",
        RangMain::Carre => "Carre",
        RangMain::QuinteFlush => "Quinte flush",
    }
}

fn evaluer_holdem(cartes: &[Carte]) -> MainEvaluee {
    let n = cartes.len();
    let mut best = MainEvaluee {
        rang: RangMain::CarteHaute,
        departage: vec![0],
    };
    if n < 5 {
        return best;
    }

    for a in 0..(n - 4) {
        for b in (a + 1)..(n - 3) {
            for c in (b + 1)..(n - 2) {
                for d in (c + 1)..(n - 1) {
                    for e in (d + 1)..n {
                        let main = [cartes[a], cartes[b], cartes[c], cartes[d], cartes[e]];
                        let ev = evaluer_5(&main);
                        if ev > best {
                            best = ev;
                        }
                    }
                }
            }
        }
    }
    best
}

fn evaluer_5(main: &[Carte; 5]) -> MainEvaluee {
    let mut valeurs: Vec<u8> = main.iter().map(|c| c.valeur.en_u8()).collect();
    valeurs.sort_unstable_by(|a, b| b.cmp(a));

    let couleur = main.iter().all(|c| c.couleur == main[0].couleur);
    let suite_haute = suite_haute(&valeurs);

    let mut freqs: BTreeMap<u8, u8> = BTreeMap::new();
    for v in &valeurs {
        *freqs.entry(*v).or_insert(0) += 1;
    }

    let mut groupes = freqs.into_iter().map(|(v, n)| (n, v)).collect::<Vec<_>>();
    groupes.sort_by(|a, b| b.cmp(a));

    if couleur {
        if let Some(h) = suite_haute {
            return MainEvaluee {
                rang: RangMain::QuinteFlush,
                departage: vec![h],
            };
        }
    }

    if groupes[0].0 == 4 {
        return MainEvaluee {
            rang: RangMain::Carre,
            departage: vec![groupes[0].1, groupes[1].1],
        };
    }

    if groupes[0].0 == 3 && groupes[1].0 == 2 {
        return MainEvaluee {
            rang: RangMain::Full,
            departage: vec![groupes[0].1, groupes[1].1],
        };
    }

    if couleur {
        return MainEvaluee {
            rang: RangMain::Couleur,
            departage: valeurs,
        };
    }

    if let Some(h) = suite_haute {
        return MainEvaluee {
            rang: RangMain::Suite,
            departage: vec![h],
        };
    }

    if groupes[0].0 == 3 {
        let brelan = groupes[0].1;
        let mut kickers = groupes
            .iter()
            .filter(|(n, _)| *n == 1)
            .map(|(_, v)| *v)
            .collect::<Vec<_>>();
        kickers.sort_unstable_by(|a, b| b.cmp(a));
        let mut d = vec![brelan];
        d.extend(kickers);
        return MainEvaluee {
            rang: RangMain::Brelan,
            departage: d,
        };
    }

    if groupes[0].0 == 2 && groupes[1].0 == 2 {
        let p1 = groupes[0].1.max(groupes[1].1);
        let p2 = groupes[0].1.min(groupes[1].1);
        let k = groupes
            .iter()
            .find(|(n, _)| *n == 1)
            .map(|(_, v)| *v)
            .unwrap_or(0);
        return MainEvaluee {
            rang: RangMain::DoublePaire,
            departage: vec![p1, p2, k],
        };
    }

    if groupes[0].0 == 2 {
        let paire = groupes[0].1;
        let mut kickers = groupes
            .iter()
            .filter(|(n, _)| *n == 1)
            .map(|(_, v)| *v)
            .collect::<Vec<_>>();
        kickers.sort_unstable_by(|a, b| b.cmp(a));
        let mut d = vec![paire];
        d.extend(kickers);
        return MainEvaluee {
            rang: RangMain::Paire,
            departage: d,
        };
    }

    MainEvaluee {
        rang: RangMain::CarteHaute,
        departage: valeurs,
    }
}

fn suite_haute(valeurs_desc: &[u8]) -> Option<u8> {
    let mut u = valeurs_desc.to_vec();
    u.sort_unstable();
    u.dedup();
    if u.len() != 5 {
        return None;
    }

    if u == [2, 3, 4, 5, 14] {
        return Some(5);
    }

    let min = u[0];
    let max = u[4];
    if max - min == 4 {
        Some(max)
    } else {
        None
    }
}
