use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

// ─── États ──────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EtatMines {
    EnAttente,
    Actif,
    Gagne(f64),
    Perdu,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaseMine {
    Cachee,
    Revelee,       // gemme trouvée
    MineRevelee,   // mine (fin de partie)
    MineMontree,   // mine révélée après fin (pas clickée)
}

// ─── Jeu ────────────────────────────────────────────────────────────

pub struct JeuMines {
    pub grille: [[CaseMine; 5]; 5],
    pub nb_mines: u8,
    pub mise: f64,
    pub multiplicateur: f64,
    pub cases_revelees: u8,
    pub etat: EtatMines,
    pub message: String,

    // provably fair
    pub graine_serveur: String,
    pub hash_graine_serveur: String,
    pub graine_client: String,
    pub nonce: u64,
    positions_mines: Vec<(usize, usize)>,

    // autoplay
    pub mode_autoplay: bool,
    pub autoplay_restantes: u8,
}

impl JeuMines {
    /// Crée une nouvelle partie. Valide les entrées.
    pub fn nouveau(
        nb_mines: u8,
        mise: f64,
        graine_client: String,
        graine_serveur: String,
        nonce: u64,
    ) -> Result<Self, String> {
        if nb_mines < 1 || nb_mines > 24 {
            return Err("Le nombre de mines doit être entre 1 et 24.".into());
        }
        if mise <= 0.0 {
            return Err("La mise doit être supérieure à 0.".into());
        }

        let hash_graine_serveur = hex_sha256(&graine_serveur);
        let positions_mines = generer_positions_mines(
            &graine_serveur,
            &graine_client,
            nonce,
            nb_mines,
        );

        Ok(Self {
            grille: [[CaseMine::Cachee; 5]; 5],
            nb_mines,
            mise,
            multiplicateur: 1.0,
            cases_revelees: 0,
            etat: EtatMines::Actif,
            message: "Partie lancée ! Cliquez sur une case.".into(),
            graine_serveur,
            hash_graine_serveur,
            graine_client,
            nonce,
            positions_mines,
            mode_autoplay: false,
            autoplay_restantes: 0,
        })
    }

    /// Révèle une case (ligne, col) — renvoie le nouveau multiplicateur ou erreur.
    pub fn reveler(&mut self, ligne: usize, col: usize) -> Result<f64, String> {
        if self.etat != EtatMines::Actif {
            return Err("La partie n'est pas active.".into());
        }
        if ligne >= 5 || col >= 5 {
            return Err("Position invalide (0-4).".into());
        }
        if self.grille[ligne][col] != CaseMine::Cachee {
            return Err("Cette case est déjà révélée.".into());
        }

        if self.est_mine(ligne, col) {
            self.grille[ligne][col] = CaseMine::MineRevelee;
            self.etat = EtatMines::Perdu;
            self.message = format!("💥 Mine ! Vous perdez {:.2}.", self.mise);
            self.reveler_tout();
            return Ok(0.0);
        }

        self.grille[ligne][col] = CaseMine::Revelee;
        self.cases_revelees += 1;

        // Calcul du multiplicateur suivant
        self.multiplicateur = self.calculer_multiplicateur();
        let paiement = self.mise * self.multiplicateur;
        self.message = format!(
            "💎 Gemme ! Multiplicateur: {:.4}x — Paiement potentiel: {:.2}",
            self.multiplicateur, paiement
        );

        // Toutes les cases sûres révélées → gain automatique
        let total_sures = 25 - self.nb_mines as u8;
        if self.cases_revelees >= total_sures {
            let paiement_final = self.mise * self.multiplicateur;
            self.etat = EtatMines::Gagne(paiement_final);
            self.message = format!(
                "🎉 Toutes les gemmes ! Paiement max: {:.2} ({:.4}x)",
                paiement_final, self.multiplicateur
            );
            self.reveler_tout();
        }

        Ok(self.multiplicateur)
    }

    /// Le joueur encaisse son gain actuel.
    pub fn encaisser(&mut self) -> Result<f64, String> {
        if self.etat != EtatMines::Actif {
            return Err("La partie n'est pas active.".into());
        }
        if self.cases_revelees == 0 {
            return Err("Vous devez révéler au moins une case avant d'encaisser.".into());
        }

        let paiement = self.mise * self.multiplicateur;
        self.etat = EtatMines::Gagne(paiement);
        self.message = format!(
            "💰 Encaissé ! Paiement: {:.2} ({:.4}x)",
            paiement, self.multiplicateur
        );
        self.reveler_tout();
        Ok(paiement)
    }

    /// Autoplay : révèle automatiquement `n` cases, s'arrête sur mine.
    pub fn autoplay(&mut self, n: u8) {
        if self.etat != EtatMines::Actif {
            return;
        }
        let mut restant = n;
        for ligne in 0..5 {
            for col in 0..5 {
                if restant == 0 || self.etat != EtatMines::Actif {
                    return;
                }
                if self.grille[ligne][col] == CaseMine::Cachee {
                    let _ = self.reveler(ligne, col);
                    restant -= 1;
                }
            }
        }
    }

    /// Calcule le multiplicateur cumulé après `cases_revelees` gemmes.
    /// Formule : produit de (25 - i - mines) / (25 - i) pour i de 0 à cases_revelees - 1
    /// avec house_edge = 0.99 (1% avantage maison)
    fn calculer_multiplicateur(&self) -> f64 {
        let n = 25u32;
        let m = self.nb_mines as u32;
        let mut mult = 1.0_f64;
        for i in 0..self.cases_revelees as u32 {
            let restant_total = n - i;
            let restant_sures = n - i - m;
            if restant_total == 0 || restant_sures == 0 {
                break;
            }
            mult *= restant_total as f64 / restant_sures as f64;
        }
        // Appliquer le house edge de 1%
        mult * 0.99
    }

    /// Vérifie si (ligne, col) est une mine.
    fn est_mine(&self, ligne: usize, col: usize) -> bool {
        self.positions_mines.contains(&(ligne, col))
    }

    /// Révèle toutes les mines (après fin de partie).
    fn reveler_tout(&mut self) {
        for &(l, c) in &self.positions_mines.clone() {
            if self.grille[l][c] == CaseMine::Cachee {
                self.grille[l][c] = CaseMine::MineMontree;
            }
        }
    }

    /// Permet la vérification d'équité : recalcule les positions depuis les graines.
    pub fn verifier_equite(&self) -> bool {
        let recalcul = generer_positions_mines(
            &self.graine_serveur,
            &self.graine_client,
            self.nonce,
            self.nb_mines,
        );
        recalcul == self.positions_mines
    }

    pub fn est_termine(&self) -> bool {
        matches!(self.etat, EtatMines::Gagne(_) | EtatMines::Perdu)
    }

    pub fn paiement(&self) -> f64 {
        match self.etat {
            EtatMines::Gagne(p) => p,
            _ => 0.0,
        }
    }
}

// ─── Provably Fair ──────────────────────────────────────────────────

/// Génère les positions des mines via HMAC-SHA256 (Fisher-Yates déterministe).
fn generer_positions_mines(
    graine_serveur: &str,
    graine_client: &str,
    nonce: u64,
    nb_mines: u8,
) -> Vec<(usize, usize)> {
    // HMAC-SHA256(key = server_seed, data = client_seed:nonce:round)
    let mut indices: Vec<usize> = (0..25).collect();
    
    for i in 0..nb_mines as usize {
        let data = format!("{}:{}:{}", graine_client, nonce, i);
        let mut mac = HmacSha256::new_from_slice(graine_serveur.as_bytes())
            .expect("HMAC accepte toute taille de clé");
        mac.update(data.as_bytes());
        let result = mac.finalize().into_bytes();

        // Prendre les 4 premiers octets comme u32
        let val = u32::from_be_bytes([result[0], result[1], result[2], result[3]]);
        let remaining = indices.len() - i;
        let j = i + (val as usize % remaining);
        indices.swap(i, j);
    }

    indices[..nb_mines as usize]
        .iter()
        .map(|&idx| (idx / 5, idx % 5))
        .collect()
}

/// SHA-256 hex d'une chaîne (pour le hash public de la graine serveur).
fn hex_sha256(input: &str) -> String {
    use sha2::Digest;
    let mut hasher = sha2::Sha256::new();
    hasher.update(input.as_bytes());
    let result = hasher.finalize();
    result.iter().map(|b| format!("{:02x}", b)).collect()
}
