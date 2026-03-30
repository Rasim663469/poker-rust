use crate::core::cards::{Carte, Paquet, Valeur};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HiLoGuess {
    Higher,
    Lower,
    Equal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HiLoState {
    EnAttenteMise,
    EnAttenteChoix,
    Resultat,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AceMode {
    High,
    Low,
}

#[derive(Clone, Debug)]
pub struct HiLoOutcome {
    pub win: bool,
    pub tie: bool,
    pub current: Carte,
    pub next: Carte,
    pub payout: u32,
}

#[derive(Clone, Debug)]
pub struct HiLoHistoryEntry {
    pub current: Carte,
    pub next: Carte,
    pub guess: HiLoGuess,
    pub win: bool,
    pub tie: bool,
    pub payout: u32,
}

#[derive(Clone, Debug)]
pub struct HiLoConfig {
    pub allow_equal: bool,
    pub ace_mode: AceMode,
    pub payout_win: u32,
    pub payout_equal: u32,
    pub min_bet: u32,
    pub max_bet: u32,
}

impl Default for HiLoConfig {
    fn default() -> Self {
        Self {
            allow_equal: false,
            ace_mode: AceMode::High,
            payout_win: 1,
            payout_equal: 5,
            min_bet: 1,
            max_bet: 1000,
        }
    }
}

pub struct HiLoGame {
    paquet: Paquet,
    pub jetons: u32,
    pub mise: u32,
    pub current: Option<Carte>,
    pub next: Option<Carte>,
    pub etat: HiLoState,
    pub message: String,
    pub config: HiLoConfig,
    pub history: Vec<HiLoHistoryEntry>,
    pub streak: u32,
    pub last_outcome: Option<HiLoOutcome>,
}

impl HiLoGame {
    pub fn new(jetons_depart: u32) -> Self {
        Self::new_with_config(jetons_depart, HiLoConfig::default())
    }

    pub fn new_with_config(jetons_depart: u32, config: HiLoConfig) -> Self {
        let mut paquet = Paquet::nouveau();
        paquet.melanger();
        let current = paquet.tirer_carte();
        Self {
            paquet,
            jetons: jetons_depart,
            mise: 0,
            current,
            next: None,
            etat: HiLoState::EnAttenteMise,
            message: "Place une mise pour commencer.".to_string(),
            config,
            history: Vec::new(),
            streak: 0,
            last_outcome: None,
        }
    }

    pub fn start_round(&mut self, mise: u32) -> Result<(), String> {
        if self.jetons == 0 {
            return Err("Tu n'as plus de jetons.".to_string());
        }
        if mise < self.config.min_bet || mise > self.config.max_bet {
            return Err(format!(
                "Mise invalide. Limites: {}..={}",
                self.config.min_bet, self.config.max_bet
            ));
        }
        if mise > self.jetons {
            return Err("Mise invalide.".to_string());
        }
        if self.paquet.cartes.len() < 10 {
            self.paquet = Paquet::nouveau();
            self.paquet.melanger();
        }
        if self.current.is_none() {
            self.current = self.paquet.tirer_carte();
        }
        self.mise = mise;
        self.next = None;
        self.etat = HiLoState::EnAttenteChoix;
        self.message = "Choisis Hi ou Lo.".to_string();
        Ok(())
    }

    pub fn rebet(&mut self) -> Result<(), String> {
        if self.mise == 0 {
            return Err("Aucune mise precedente.".to_string());
        }
        self.start_round(self.mise)
    }

    pub fn guess(&mut self, guess: HiLoGuess) -> Result<HiLoOutcome, String> {
        if self.etat != HiLoState::EnAttenteChoix {
            return Err("Pas de choix attendu.".to_string());
        }
        if guess == HiLoGuess::Equal && !self.config.allow_equal {
            return Err("Le choix Equal n'est pas autorise.".to_string());
        }
        let current = self
            .current
            .ok_or_else(|| "Carte courante manquante".to_string())?;
        let next = self
            .paquet
            .tirer_carte()
            .ok_or_else(|| "Plus de cartes.".to_string())?;

        let c = card_value(current.valeur, self.config.ace_mode);
        let n = card_value(next.valeur, self.config.ace_mode);
        let tie = n == c;
        let win = if tie {
            guess == HiLoGuess::Equal
        } else if n > c {
            guess == HiLoGuess::Higher
        } else {
            guess == HiLoGuess::Lower
        };

        let payout = if tie {
            if win {
                self.mise * self.config.payout_equal
            } else {
                0
            }
        } else if win {
            self.mise * self.config.payout_win
        } else {
            0
        };

        if win {
            self.jetons += payout;
            self.streak = self.streak.saturating_add(1);
            if tie {
                self.message = format!("Egalite gagnee ! (+{})", payout);
            } else {
                self.message = format!("Gagne ! (+{})", payout);
            }
        } else {
            self.jetons = self.jetons.saturating_sub(self.mise);
            self.streak = 0;
            if tie {
                self.message = "Egalite: tu perds (regle standard).".to_string();
            } else {
                self.message = "Perdu.".to_string();
            }
        }

        self.current = Some(next);
        self.next = Some(next);
        self.etat = HiLoState::Resultat;

        let outcome = HiLoOutcome {
            win,
            tie,
            current,
            next,
            payout,
        };
        self.last_outcome = Some(outcome.clone());
        self.history.push(HiLoHistoryEntry {
            current,
            next,
            guess,
            win,
            tie,
            payout,
        });
        if self.history.len() > 20 {
            self.history.remove(0);
        }

        Ok(outcome)
    }

    pub fn reset_round(&mut self) {
        self.mise = 0;
        self.next = None;
        self.etat = HiLoState::EnAttenteMise;
        if self.current.is_none() {
            self.current = self.paquet.tirer_carte();
        }
        self.message = "Place une mise pour commencer.".to_string();
    }
}

fn card_value(v: Valeur, ace_mode: AceMode) -> u8 {
    match v {
        Valeur::As => match ace_mode {
            AceMode::High => 14,
            AceMode::Low => 1,
        },
        _ => v.en_u8(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ace_mode_changes_value() {
        assert_eq!(card_value(Valeur::As, AceMode::High), 14);
        assert_eq!(card_value(Valeur::As, AceMode::Low), 1);
    }

    #[test]
    fn equal_is_loss_when_not_allowed() {
        let mut game = HiLoGame::new_with_config(
            100,
            HiLoConfig {
                allow_equal: false,
                ..HiLoConfig::default()
            },
        );
        game.current = Some(Carte {
            valeur: Valeur::Dix,
            couleur: crate::core::cards::Couleur::Coeur,
        });
        game.etat = HiLoState::EnAttenteChoix;
        game.paquet.cartes.push(Carte {
            valeur: Valeur::Dix,
            couleur: crate::core::cards::Couleur::Pique,
        });
        let res = game.guess(HiLoGuess::Higher).unwrap();
        assert!(res.tie);
        assert!(!res.win);
    }

    #[test]
    fn equal_can_win_when_allowed() {
        let mut game = HiLoGame::new_with_config(
            100,
            HiLoConfig {
                allow_equal: true,
                payout_equal: 5,
                ..HiLoConfig::default()
            },
        );
        game.current = Some(Carte {
            valeur: Valeur::Dix,
            couleur: crate::core::cards::Couleur::Coeur,
        });
        game.etat = HiLoState::EnAttenteChoix;
        game.paquet.cartes.push(Carte {
            valeur: Valeur::Dix,
            couleur: crate::core::cards::Couleur::Pique,
        });
        game.mise = 10;
        let res = game.guess(HiLoGuess::Equal).unwrap();
        assert!(res.tie);
        assert!(res.win);
        assert_eq!(res.payout, 50);
    }
}
