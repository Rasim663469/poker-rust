use std::cmp::Ordering;
use std::collections::BTreeMap;
use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};   

<<<<<<< HEAD:src/partie.rs
use crate::carte::{Carte, Paquet};
use crate::joueur::Joueur;
use crate::utils::demander;
use crate::communication::{MessageClient,MessageServeur,ActionJoueur};
=======
use crate::core::cards::{Carte, Paquet};
use crate::core::player::Joueur;
use crate::core::utils::{demander, demander_u32};
>>>>>>> Rasim:src/games/poker/engine.rs

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

impl RangMain {
    fn libelle(self) -> &'static str {
        match self {
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
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct MainEvaluee {
    rang: RangMain,
    departage: Vec<u8>,
}

impl MainEvaluee {
    fn departage_en_texte(&self) -> String {
        self.departage.iter()
            .map(|&v| match v {
                14 => "A".to_string(),
                13 => "R".to_string(),
                12 => "D".to_string(),
                11 => "V".to_string(),
                n => n.to_string(),
            })
            .collect::<Vec<_>>()
            .join(", ")
    }
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
    pub sockets: Vec<TcpStream>,
    paquet: Paquet,
    cartes_communes: Vec<Carte>,
    pot: u32,
    contributions: Vec<u32>,
    dealer_idx: usize,
    small_blind: u32,
    big_blind: u32,
}

impl Partie {
    pub fn nouvelle(noms: Vec<String>, jetons_depart: u32, small_blind: u32, big_blind: u32, sockets : Vec<TcpStream>) -> Self {
        let joueurs: Vec<Joueur> = noms
            .into_iter()
            .map(|nom| Joueur::nouveau(nom, jetons_depart))
            .collect();

        Self {
            contributions: vec![0; joueurs.len()],
            joueurs,
            sockets,
            paquet: tokio::task::block_in_place(|| Paquet::nouveau()),
            cartes_communes: Vec::new(),
            pot: 0,
            dealer_idx: 0,
            small_blind,
            big_blind,
        }
    }

    pub async fn diffuser(&mut self, message: &MessageServeur) -> tokio::io::Result<()> {
        let data = serde_json::to_vec(message).expect("Erreur sérialisation broadcast");

        for socket in &mut self.sockets {
            if let Err(e) = socket.write_all(&data).await {
                eprintln!("[WARN] Diffusion échouée pour un joueur (déconnecté ?) : {}", e);
            }
        }
        Ok(())
    }

    pub async fn jouer_session_cli(&mut self) -> tokio::io::Result<()>{
        loop {
            if self.nb_joueurs_avec_jetons() < 2 {
                println!("Session terminee: moins de 2 joueurs ont encore des jetons.");
                break;
            }

            self.jouer_manche_holdem_cli().await?;

            let continuer = demander("\nNouvelle main ? (o/n): ")
                .ok()
                .map(|s:String| s.eq_ignore_ascii_case("o"))
                .unwrap_or(false);

            if !continuer {
                break;
            }
        }
        Ok(())
    }

    pub async fn jouer_manche_holdem_cli(&mut self) -> tokio::io::Result<()>{
        self.preparer_manche();

        let participants = self.indices_participants();
        if participants.len() < 2 {
            println!("Pas assez de joueurs avec des jetons pour lancer une main.");
            return Ok(());
        }

        let (sb_idx, bb_idx) = self.indices_blinds();
        println!("\n=== Nouvelle main (Texas Hold'em) ===");
        println!("Donneur: {}", self.joueurs[self.dealer_idx].nom);
        println!(
            "Small blind: {} ({}), Big blind: {} ({})",
            self.joueurs[sb_idx].nom, self.small_blind, self.joueurs[bb_idx].nom, self.big_blind
        );

        let sb_posee = self.prelever_blind(sb_idx, self.small_blind);
        let bb_posee = self.prelever_blind(bb_idx, self.big_blind);
        let mise_depart = sb_posee.max(bb_posee);
        println!("Pot initial (blinds): {}", self.pot);

        self.distribuer_pocket().await?;
        println!("\n=== Pocket cards ===");
        for idx in &participants {
            self.joueurs[*idx].afficher_main(true);
        }

        let start_preflop = self.prochain_participant_apres(bb_idx).unwrap_or(bb_idx);
        self.tour_mises("Preflop", start_preflop, mise_depart).await?;
        if self.main_terminee_par_abandon().await? {
            self.avancer_donneur();
            return Ok(());
        }
        self.reinitialiser_mises_tour();

        self.bruler_une_carte();
        self.distribuer_communes(3);
        println!("\n=== Flop ===");
        self.afficher_communes();
        self.diffuser_table().await?;
        let start_postflop = self
            .prochain_participant_apres(self.dealer_idx)
            .unwrap_or(self.dealer_idx);
        self.tour_mises("Flop", start_postflop, 0).await?;
        if self.main_terminee_par_abandon().await? {
            self.avancer_donneur();
            return Ok(());
        }
        self.reinitialiser_mises_tour();

        self.bruler_une_carte();
        self.distribuer_communes(1);
        println!("\n=== Turn ===");
        self.afficher_communes();
        self.diffuser_table().await?;
        self.tour_mises("Turn", start_postflop, 0).await?;
        if self.main_terminee_par_abandon().await? {
            self.avancer_donneur();
            return Ok(());
        }
        self.reinitialiser_mises_tour();

        self.bruler_une_carte();
        self.distribuer_communes(1);
        println!("\n=== River ===");
        self.afficher_communes();
        self.diffuser_table().await?;
        self.tour_mises("River", start_postflop, 0).await?;
        if self.main_terminee_par_abandon().await? {
            self.avancer_donneur();
            return Ok(());
        }
        self.reinitialiser_mises_tour();

        self.showdown().await?;
        self.avancer_donneur();
        Ok(())
    }

    fn preparer_manche(&mut self) {
        tokio::task::block_in_place(|| {
            self.paquet = Paquet::nouveau();
            self.paquet.melanger();
        });
        self.cartes_communes.clear();
        self.pot = 0;
        self.contributions.fill(0);
        for joueur in &mut self.joueurs {
            joueur.main.clear();
            joueur.couche = joueur.jetons == 0;
            joueur.mise_tour = 0;
        }
    }

    fn indices_participants(&self) -> Vec<usize> {
        self.joueurs
            .iter()
            .enumerate()
            .filter_map(|(i, j)| if j.jetons > 0 { Some(i) } else { None })
            .collect()
    }

    fn nb_joueurs_avec_jetons(&self) -> usize {
        self.joueurs.iter().filter(|j| j.jetons > 0).count()
    }

    fn nb_actifs_non_couches(&self) -> usize {
        self.joueurs.iter().filter(|j| !j.couche).count()
    }

    fn indices_blinds(&self) -> (usize, usize) {
        let participants = self.indices_participants();
        if participants.len() == 2 {
            let dealer = self.dealer_idx;
            let autre = self.prochain_participant_apres(dealer).unwrap_or(dealer);
            return (dealer, autre);
        }

        let sb = self
            .prochain_participant_apres(self.dealer_idx)
            .unwrap_or(self.dealer_idx);
        let bb = self.prochain_participant_apres(sb).unwrap_or(sb);
        (sb, bb)
    }

    fn prelever_blind(&mut self, idx: usize, montant: u32) -> u32 {
        self.investir(idx, montant)
    }

    fn investir(&mut self, idx: usize, montant: u32) -> u32 {
        let paye = self.joueurs[idx].jetons.min(montant);
        self.joueurs[idx].jetons -= paye;
        self.joueurs[idx].mise_tour += paye;
        self.contributions[idx] += paye;
        self.pot += paye;
        paye
    }

    pub async fn distribuer_pocket(&mut self) -> tokio::io::Result<()> {
        for _ in 0..2 {
            let len = self.joueurs.len();
            for i in 0..len {
                if self.joueurs[i].jetons > 0 || self.joueurs[i].mise_tour > 0 {
                    let carte_option = tokio::task::block_in_place(|| self.paquet.tirer_carte());
                    if let Some(carte) = carte_option {
                        self.joueurs[i].main.push(carte);
                    }
                }
            }
        }
        
        for i in 0..self.joueurs.len() {
            if self.joueurs[i].jetons > 0 || self.joueurs[i].mise_tour > 0 {
                let msg = MessageServeur::MesCartes { cartes: self.joueurs[i].main.clone() };
                let data = serde_json::to_vec(&msg).unwrap();
                if let Err(e) = self.sockets[i].write_all(&data).await {
                    eprintln!("[WARN] Impossible d'envoyer les cartes à {} : {}", self.joueurs[i].nom, e);
                    self.joueurs[i].couche = true;
                }
            }
        }
        Ok(())
    }

    fn bruler_une_carte(&mut self) {
        tokio::task::block_in_place(|| {
            let _ = self.paquet.tirer_carte();
        });
    }

    fn distribuer_communes(&mut self, n: usize) {
        tokio::task::block_in_place(|| {
            for _ in 0..n {
                if let Some(carte) = self.paquet.tirer_carte() {
                    self.cartes_communes.push(carte);
                }
            }
        });
    }

    pub async fn diffuser_table(&mut self) -> tokio::io::Result<()> {
        let msg = MessageServeur::MajTable {
            pot: self.pot,
            cartes_communes: self.cartes_communes.clone(),
        };
        self.diffuser(&msg).await
    }

    fn afficher_communes(&self) {
        let cartes = self
            .cartes_communes
            .iter()
            .map(|c| c.to_string())
            .collect::<Vec<_>>()
            .join(" ");
        println!("Cartes communes: {}", cartes);
        println!("Pot: {}", self.pot);
    }

    pub async fn tour_mises(&mut self, nom_tour: &str, start_idx: usize, mise_actuelle_depart: u32) -> tokio::io::Result<()> {
        println!("\n--- Tour de mise: {} ---", nom_tour);

        let mut mise_actuelle = mise_actuelle_depart;
        let mut taille_relance_complete = self.big_blind.max(1);
        let mut compteur_relances_completes = 0u32;

        let mut a_jouer = vec![false; self.joueurs.len()];
        let mut a_deja_joue = vec![false; self.joueurs.len()];
        let mut dernier_compteur_vu = vec![0u32; self.joueurs.len()];

        for (i, joueur) in self.joueurs.iter().enumerate() {
            if !joueur.couche && joueur.jetons > 0 {
                a_jouer[i] = true;
            }
        }

        let mut idx = start_idx;
        while self.nb_actifs_non_couches() > 1 && a_jouer.iter().any(|&b| b) {
            if idx >= self.joueurs.len() {
                idx = 0;
            }

            if !a_jouer[idx] || self.joueurs[idx].couche || self.joueurs[idx].jetons == 0 {
                idx = (idx + 1) % self.joueurs.len();
                continue;
            }

            let to_call = mise_actuelle.saturating_sub(self.joueurs[idx].mise_tour);
            
            let total_max = self.joueurs[idx].mise_tour + self.joueurs[idx].jetons;
            let total_min_raise = if mise_actuelle == 0 {
                self.big_blind
            } else {
                mise_actuelle + taille_relance_complete
            };
            let a_action_rouverte = !a_deja_joue[idx] || dernier_compteur_vu[idx] < compteur_relances_completes;
            let peut_relancer = self.joueurs[idx].jetons > to_call
                && total_max >= total_min_raise
                && a_action_rouverte;
            
            let msg_demande = MessageServeur::DemanderAction { 
                to_call, 
                peut_relancer,
                jetons_restants: self.joueurs[idx].jetons
            };
            
            let data = serde_json::to_vec(&msg_demande).unwrap();
            if let Err(e) = self.sockets[idx].write_all(&data).await {
                eprintln!("[WARN] Impossible de contacter {} : {} → fold automatique", self.joueurs[idx].nom, e);
                self.joueurs[idx].couche = true;
                a_jouer[idx] = false;
                idx = (idx + 1) % self.joueurs.len();
                continue;
            }

            let mut tampon = [0u8; 4096];
            let lecture = self.sockets[idx].read(&mut tampon).await;
            let nb_octets = match lecture {
                Ok(0) => None,
                Ok(n) => Some(n),
                Err(_) => None,
            };
            if nb_octets.is_none() {
                eprintln!("[WARN] {} injoignable ou déconnecté → fold automatique", self.joueurs[idx].nom);
                self.joueurs[idx].couche = true;
                a_jouer[idx] = false;
                idx = (idx + 1) % self.joueurs.len();
                continue;
            }
            let n = nb_octets.unwrap();

            let parsed: Option<MessageClient> = serde_json::from_slice(&tampon[..n]).ok();
            if parsed.is_none() {
                eprintln!("[WARN] Message invalide de {} → fold automatique", self.joueurs[idx].nom);
                self.joueurs[idx].couche = true;
                a_jouer[idx] = false;
                idx = (idx + 1) % self.joueurs.len();
                continue;
            }
            let reponse = parsed.unwrap();

            let mut nouvelle_mise_totale: Option<u32> = None;

            if let MessageClient::Action(action) = reponse {
                let nom_joueur = self.joueurs[idx].nom.clone();
                match action {
                    ActionJoueur::Fold => {
                        self.joueurs[idx].couche = true;
                        self.diffuser(&MessageServeur::Bienvenue { message: format!("\n[INFO ACTION] {} s'est couché.\n", nom_joueur) }).await?;
                    }
                    ActionJoueur::Call => {
                        let paiement = self.joueurs[idx].jetons.min(to_call);
                        let _ = self.investir(idx, paiement);
                        let msg = if to_call == 0 {
                            format!("\n[INFO ACTION] {} a check.\n", nom_joueur)
                        } else {
                            format!("\n[INFO ACTION] {} a suivi ({} jetons).\n", nom_joueur, paiement)
                        };
                        self.diffuser(&MessageServeur::Bienvenue { message: msg }).await?;
                    }
                    ActionJoueur::Raise(total) => {
                        let paiement = total.saturating_sub(self.joueurs[idx].mise_tour);
                        let _ = self.investir(idx, paiement);
                        nouvelle_mise_totale = Some(total);
                        self.diffuser(&MessageServeur::Bienvenue { message: format!("\n[INFO ACTION] {} a relancé à {} jetons.\n", nom_joueur, total) }).await?;
                    }
                    _ => {}
                }
            }

            a_deja_joue[idx] = true;
            a_jouer[idx] = false;

            if let Some(nouvelle_mise) = nouvelle_mise_totale {
                if nouvelle_mise > mise_actuelle {
                    let augmentation = nouvelle_mise - mise_actuelle;
                    let relance_complete = augmentation >= taille_relance_complete;
                    mise_actuelle = nouvelle_mise;

                    if relance_complete {
                        taille_relance_complete = augmentation;
                        compteur_relances_completes += 1;
                    }

                    for (i, joueur) in self.joueurs.iter().enumerate() {
                        if i == idx || joueur.couche || joueur.jetons == 0 {
                            continue;
                        }
                        if joueur.mise_tour < mise_actuelle {
                            a_jouer[i] = true;
                        }
                    }
                }
            }
            dernier_compteur_vu[idx] = compteur_relances_completes;

            idx = (idx + 1) % self.joueurs.len();
        }
        Ok(())
    }
    
    fn demander_action(&self, idx: usize, to_call: u32, peut_relancer: bool) -> String {
        loop {
            let joueur = &self.joueurs[idx];

            let prompt = if to_call == 0 {
                if peut_relancer {
                    "Action [c=check, r=relance, a=all-in, f=fold]: "
                } else {
                    "Action [c=check, a=all-in, f=fold]: "
                }
            } else if joueur.jetons > to_call {
                if peut_relancer {
                    "Action [s=suivre, r=relance, a=all-in, f=fold]: "
                } else {
                    "Action [s=suivre, a=all-in, f=fold]: "
                }
            } else if joueur.jetons == to_call {
                "Action [s=suivre(all-in), a=all-in, f=fold]: "
            } else {
                "Action [a=all-in partiel, f=fold]: "
            };

            let entree = demander(prompt)
                .ok()
                .unwrap_or_default()
                .to_lowercase();
            match entree.as_str() {
                "f" => return "f".to_string(),
                "c" if to_call == 0 => return "c".to_string(),
                "s" if to_call > 0 && joueur.jetons >= to_call => return "s".to_string(),
                "a" if joueur.jetons > 0 => return "a".to_string(),
                "r" if peut_relancer => return "r".to_string(),
                _ => println!("Action invalide."),
            }
        }
    }

    fn reinitialiser_mises_tour(&mut self) {
        for joueur in &mut self.joueurs {
            joueur.mise_tour = 0;
        }
    }

pub async fn main_terminee_par_abandon(&mut self) -> tokio::io::Result<bool> {
    if self.nb_actifs_non_couches() != 1 {
        return Ok(false);
    }

    let gagnant = self
        .joueurs
        .iter()
        .enumerate()
        .find_map(|(i, j)| if !j.couche { Some(i) } else { None });

    if let Some(idx) = gagnant {
        let annonce = format!(
            "\nTous les autres joueurs se sont couchés. {} gagne le pot de {} jetons.",
            self.joueurs[idx].nom, self.pot
        );
        self.joueurs[idx].jetons += self.pot;
        self.pot = 0;
        self.diffuser(&MessageServeur::Bienvenue { message: annonce }).await?;
    }

    Ok(true)
}

    pub async fn showdown(&mut self) -> tokio::io::Result<()> {
        self.diffuser(&MessageServeur::Bienvenue { 
            message: "\n=== SHOWDOWN ===".to_string() 
        }).await?;
        self.afficher_communes();
        self.diffuser_table().await?;

        let mut evaluations: Vec<Option<MainEvaluee>> = vec![None; self.joueurs.len()];
        for idx in 0..self.joueurs.len() {
            if self.joueurs[idx].couche {
                continue;
            }
            let mut cartes = Vec::with_capacity(7);
            cartes.extend(self.joueurs[idx].main.iter().copied());
            cartes.extend(self.cartes_communes.iter().copied());
            let eval = evaluer_meilleure_main(&cartes);
            
            let msg = format!("{} montre : {} (Kickers: {})", 
                self.joueurs[idx].nom, 
                eval.rang.libelle(), 
                eval.departage_en_texte()
            );
            
            self.diffuser(&MessageServeur::Bienvenue { message: msg }).await?;
            evaluations[idx] = Some(eval);
        }

        if evaluations.iter().all(|x| x.is_none()) {
            return Ok(());
        }

        let mut restants = self.contributions.clone();
        let mut _total_distribue = 0u32;

        while let Some(niveau) = restants.iter().copied().filter(|&c| c > 0).min() {
            let participants_pot: Vec<usize> = restants
                .iter()
                .enumerate()
                .filter_map(|(i, &c)| if c > 0 { Some(i) } else { None })
                .collect();
            let montant_pot = niveau * participants_pot.len() as u32;

            for idx in &participants_pot {
                restants[*idx] -= niveau;
            }

            let eligibles: Vec<usize> = participants_pot
                .iter()
                .copied()
                .filter(|&i| !self.joueurs[i].couche)
                .collect();

            if eligibles.is_empty() {
                continue;
            }

            let meilleure = eligibles
                .iter()
                .filter_map(|&i| evaluations[i].as_ref())
                .max()
                .expect("Une meilleure main doit exister pour ce pot")
                .clone();

            let gagnants: Vec<usize> = eligibles
                .iter()
                .copied()
                .filter(|&i| evaluations[i].as_ref() == Some(&meilleure))
                .collect();

            let part = montant_pot / gagnants.len() as u32;
            let reste = montant_pot % gagnants.len() as u32;
            for (k, idx) in gagnants.iter().enumerate() {
                let bonus = if (k as u32) < reste { 1 } else { 0 };
                self.joueurs[*idx].jetons += part + bonus;
            }

            let noms = gagnants
                .iter()
                .map(|&i| self.joueurs[i].nom.clone())
                .collect::<Vec<_>>()
                .join(", ");
                
            let msg_gagnant = format!(
                "Pot {} -> gagnant(s): {} ({})",
                montant_pot,
                noms,
                meilleure.rang.libelle()
            );
            self.diffuser(&MessageServeur::Bienvenue { message: msg_gagnant }).await?;

            _total_distribue += montant_pot;
        }

        self.pot = 0;
        Ok(())
    }

    fn prochain_participant_apres(&self, idx: usize) -> Option<usize> {
        if self.joueurs.is_empty() {
            return None;
        }
        for offset in 1..=self.joueurs.len() {
            let i = (idx + offset) % self.joueurs.len();
            if self.joueurs[i].jetons > 0 {
                return Some(i);
            }
        }
        None
    }

    fn avancer_donneur(&mut self) {
        if let Some(next) = self.prochain_participant_apres(self.dealer_idx) {
            self.dealer_idx = next;
        }
    }
}

pub fn evaluer_holdem_pour_gui(cartes: &[Carte]) -> (u8, Vec<u8>, &'static str) {
    let eval = evaluer_meilleure_main(cartes);
    (eval.rang as u8, eval.departage, eval.rang.libelle())
}

fn evaluer_meilleure_main(cartes: &[Carte]) -> MainEvaluee {
    let n = cartes.len();
    let mut meilleure: Option<MainEvaluee> = None;
    for i in 0..n {
        for j in (i + 1)..n {
            for k in (j + 1)..n {
                for l in (k + 1)..n {
                    for m in (l + 1)..n {
                        let combo = [cartes[i], cartes[j], cartes[k], cartes[l], cartes[m]];
                        let eval = evaluer_main_5(&combo);
                        if meilleure.as_ref().map(|x| &eval > x).unwrap_or(true) {
                            meilleure = Some(eval);
                        }
                    }
                }
            }
        }
    }
    meilleure.expect("Au moins une combinaison de 5 cartes attendue")
}

fn evaluer_main_5(main: &[Carte; 5]) -> MainEvaluee {
    let mut valeurs: Vec<u8> = main.iter().map(|c| c.valeur.en_u8()).collect();
    valeurs.sort_unstable();

    let mut compte_par_valeur: BTreeMap<u8, u8> = BTreeMap::new();
    for v in &valeurs {
        *compte_par_valeur.entry(*v).or_insert(0) += 1;
    }

    let couleur = main.iter().all(|c| c.couleur == main[0].couleur);
    let suite_haute = detecter_suite_haute(&valeurs);

    let mut groupes: Vec<(u8, u8)> = compte_par_valeur
        .iter()
        .map(|(&valeur, &count)| (count, valeur))
        .collect();
    groupes.sort_by(|a, b| b.cmp(a));

    if couleur && suite_haute.is_some() {
        return MainEvaluee {
            rang: RangMain::QuinteFlush,
            departage: vec![suite_haute.unwrap_or(0)],
        };
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
        let mut desc = valeurs.clone();
        desc.sort_unstable_by(|a, b| b.cmp(a));
        return MainEvaluee {
            rang: RangMain::Couleur,
            departage: desc,
        };
    }
    if let Some(haute) = suite_haute {
        return MainEvaluee {
            rang: RangMain::Suite,
            departage: vec![haute],
        };
    }
    if groupes[0].0 == 3 {
        let brelan = groupes[0].1;
        let mut kickers: Vec<u8> = groupes
            .iter()
            .filter_map(|(count, valeur)| if *count == 1 { Some(*valeur) } else { None })
            .collect();
        kickers.sort_unstable_by(|a, b| b.cmp(a));
        let mut departage = vec![brelan];
        departage.extend(kickers);
        return MainEvaluee {
            rang: RangMain::Brelan,
            departage,
        };
    }
    if groupes[0].0 == 2 && groupes[1].0 == 2 {
        let paire_haute = groupes[0].1.max(groupes[1].1);
        let paire_basse = groupes[0].1.min(groupes[1].1);
        let kicker = groupes
            .iter()
            .find_map(|(count, valeur)| if *count == 1 { Some(*valeur) } else { None })
            .unwrap_or(0);
        return MainEvaluee {
            rang: RangMain::DoublePaire,
            departage: vec![paire_haute, paire_basse, kicker],
        };
    }
    if groupes[0].0 == 2 {
        let paire = groupes[0].1;
        let mut kickers: Vec<u8> = groupes
            .iter()
            .filter_map(|(count, valeur)| if *count == 1 { Some(*valeur) } else { None })
            .collect();
        kickers.sort_unstable_by(|a, b| b.cmp(a));
        let mut departage = vec![paire];
        departage.extend(kickers);
        return MainEvaluee {
            rang: RangMain::Paire,
            departage,
        };
    }

    let mut desc = valeurs;
    desc.sort_unstable_by(|a, b| b.cmp(a));
    MainEvaluee {
        rang: RangMain::CarteHaute,
        departage: desc,
    }
}

fn detecter_suite_haute(valeurs_tries: &[u8]) -> Option<u8> {
    if valeurs_tries.len() != 5 {
        return None;
    }
    let mut uniques = valeurs_tries.to_vec();
    uniques.dedup();
    if uniques.len() != 5 {
        return None;
    }

    let est_suite_normale = uniques.windows(2).all(|w| w[1] == w[0] + 1);
    if est_suite_normale {
        return uniques.last().copied();
    }
    if uniques == [2, 3, 4, 5, 14] {
        return Some(5);
    }
    None
}