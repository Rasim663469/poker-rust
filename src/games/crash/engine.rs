use rand::Rng;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EtatCrash {
    EnAttente,
    EnVol,
    Encaisse { paiement: f64 },
    Explose { a: f64 },
}

pub struct JeuCrash {
    pub etat: EtatCrash,
    pub mise: f64,
    pub multiplicateur: f64,
    pub message: String,
    pub historique: Vec<f64>,
    point_crash: f64,
    multiplicateur_vol: f64,
    temps_vol: f64,
    manche_terminee: bool,
}

impl Default for JeuCrash {
    fn default() -> Self {
        Self::nouveau()
    }
}

impl JeuCrash {
    pub fn nouveau() -> Self {
        Self {
            etat: EtatCrash::EnAttente,
            mise: 0.0,
            multiplicateur: 1.0,
            point_crash: 0.0,
            multiplicateur_vol: 1.0,
            temps_vol: 0.0,
            manche_terminee: true,
            message: String::new(),
            historique: Vec::new(),
        }
    }

    pub fn lancer_tour(&mut self, mise: f64) -> Result<(), String> {
        // Une manche de crash est simple :
        // on fige la mise au départ, on tire un point de crash caché, puis on laisse monter le vol.
        if mise <= 0.0 {
            return Err("La mise doit etre superieure a 0.".to_string());
        }
        if !self.manche_terminee {
            return Err("La manche en cours n'est pas terminee.".to_string());
        }

        self.mise = mise;
        self.multiplicateur = 1.0;
        self.multiplicateur_vol = 1.0;
        self.point_crash = tirer_point_crash();
        self.temps_vol = 0.0;
        self.manche_terminee = false;
        self.etat = EtatCrash::EnVol;
        self.message = "Vol lance. Cash out avant l'explosion.".to_string();
        Ok(())
    }

    pub fn avancer(&mut self, delta_s: f32) {
        if self.manche_terminee {
            return;
        }

        let dt = delta_s.clamp(0.0, 0.25) as f64;
        self.temps_vol += dt;

        // On a choisi une montée lisible plutôt qu'une courbe trop brutale :
        // au début le joueur a le temps de réagir, puis le risque augmente franchement.
        let t = self.temps_vol;
        let vitesse_lineaire = 0.18 + 0.018 * t;
        let boost_20x = if self.multiplicateur_vol >= 20.0 {
            0.30 + 0.09 * (self.multiplicateur_vol - 20.0)
        } else {
            0.0
        };
        let vitesse = vitesse_lineaire + boost_20x;
        self.multiplicateur_vol += vitesse * dt;
        if self.etat == EtatCrash::EnVol {
            self.multiplicateur = self.multiplicateur_vol;
        }

        if self.multiplicateur_vol >= self.point_crash {
            self.multiplicateur_vol = self.point_crash;
            self.manche_terminee = true;
            match self.etat {
                EtatCrash::EnVol => {
                    self.multiplicateur = self.point_crash;
                    self.etat = EtatCrash::Explose {
                        a: self.point_crash,
                    };
                    self.message = format!("Crash a {:.2}x. Mise perdue.", self.point_crash);
                }
                EtatCrash::Encaisse { .. } => {
                    self.message = format!(
                        "Cash out valide. Le vol a fini par crash a {:.2}x.",
                        self.point_crash
                    );
                }
                _ => {}
            }
            self.ajouter_historique(self.point_crash);
        }
    }

    pub fn encaisser(&mut self) -> Result<f64, String> {
        self.encaisser_a(self.multiplicateur_vol)
    }

    pub fn encaisser_a(&mut self, multiplicateur_cible: f64) -> Result<f64, String> {
        // Le cash out verrouille le gain au multiplicateur courant.
        // Ensuite le vol peut continuer visuellement, mais le résultat du joueur est déjà fixé.
        if self.etat != EtatCrash::EnVol {
            return Err("Aucun vol actif.".to_string());
        }

        let mult = multiplicateur_cible.clamp(1.0, self.multiplicateur_vol);
        self.multiplicateur = mult;
        let paiement = self.mise * self.multiplicateur;
        self.etat = EtatCrash::Encaisse { paiement };
        self.message = format!(
            "Cash out valide a {:.2}x -> paiement {:.2}",
            self.multiplicateur, paiement
        );
        Ok(paiement)
    }

    pub fn multiplicateur_vol(&self) -> f64 {
        self.multiplicateur_vol
    }

    pub fn point_crash_revele(&self) -> Option<f64> {
        if self.manche_terminee && self.point_crash > 0.0 {
            Some(self.point_crash)
        } else {
            None
        }
    }

    pub fn est_en_vol(&self) -> bool {
        self.etat == EtatCrash::EnVol
    }

    pub fn manche_en_cours(&self) -> bool {
        !self.manche_terminee
    }

    fn ajouter_historique(&mut self, valeur: f64) {
        self.historique.push(valeur);
        if self.historique.len() > 12 {
            let surplus = self.historique.len() - 12;
            self.historique.drain(0..surplus);
        }
    }
}

fn tirer_point_crash() -> f64 {
    // On veut beaucoup de petits crashs et quelques rares très gros multiplicateurs.
    // C'est ce qui donne au jeu son côté tendu sans rendre les gros scores impossibles.
    let mut rng = rand::thread_rng();
    let u: f64 = rng.gen_range(0.0..0.99);
    let brut = (0.99 / (1.0 - u)).max(1.01);
    brut.min(50.0)
}
