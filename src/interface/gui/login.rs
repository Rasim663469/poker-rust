use eframe::egui;
use std::sync::mpsc;

pub enum LoginResultat {
    Ok { pseudo: String, jetons: u32, db_id: i32 },
    Erreur(String),
}

pub(super) struct LoginState {
    // On garde ici seulement l'état du formulaire.
    // La vraie session connectée vit ensuite dans CasinoApp.
    pub(super) pseudo: String,
    pub(super) mot_de_passe: String,
    pub(super) inscription: bool,
    pub(super) message: String,
    pub(super) en_cours: bool,
    rx: Option<mpsc::Receiver<LoginResultat>>,
}

impl Default for LoginState {
    fn default() -> Self {
        Self {
            pseudo: String::new(),
            mot_de_passe: String::new(),
            inscription: false,
            message: String::new(),
            en_cours: false,
            rx: None,
        }
    }
}

impl super::CasinoApp {
    pub(super) fn ui_login(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        // Le résultat d'auth revient depuis un thread séparé.
        // Dès qu'on le reçoit, on bascule l'app dans un état "connecté".
        if let Some(res) = self
            .login
            .rx
            .as_ref()
            .and_then(|rx: &mpsc::Receiver<LoginResultat>| rx.try_recv().ok())
        {
            self.login.en_cours = false;
            self.login.rx = None;
            match res {
                LoginResultat::Ok { pseudo, jetons, db_id } => {
                    self.joueur_pseudo = pseudo;
                    self.joueur_db_id = Some(db_id);
                    self.banque_joueur = jetons;
                    self.banque_depart_session = jetons;
                    self.wallet_sync_status = "Actif".to_string();
                    self.ecran = super::EcranCasino::Menu;
                }
                LoginResultat::Erreur(e) => {
                    self.login.message = e;
                }
            }
        }

        let w = ui.available_width().min(400.0);
        ui.vertical_centered(|ui| {
            ui.add_space(60.0);

            egui::Frame::new()
                .fill(egui::Color32::from_rgb(18, 25, 38))
                .corner_radius(14.0)
                .inner_margin(egui::Margin::symmetric(36i8, 32i8))
                .show(ui, |ui| {
                    ui.set_max_width(w);

                    ui.heading(if self.login.inscription {
                        "🎰  Créer un compte"
                    } else {
                        "🎰  Connexion"
                    });
                    ui.add_space(18.0);

                    egui::Grid::new("login_grid")
                        .num_columns(2)
                        .spacing([12.0, 10.0])
                        .show(ui, |ui| {
                            ui.label("Pseudo :");
                            ui.add_sized(
                                [220.0, 24.0],
                                egui::TextEdit::singleline(&mut self.login.pseudo),
                            );
                            ui.end_row();

                            ui.label("Mot de passe :");
                            ui.add_sized(
                                [220.0, 24.0],
                                egui::TextEdit::singleline(&mut self.login.mot_de_passe)
                                    .password(true),
                            );
                            ui.end_row();
                        });

                    ui.add_space(10.0);

                    if !self.login.message.is_empty() {
                        ui.colored_label(egui::Color32::from_rgb(230, 80, 80), &self.login.message);
                        ui.add_space(6.0);
                    }

                    ui.horizontal(|ui| {
                        let label_btn = if self.login.inscription { "Créer le compte" } else { "Se connecter" };
                        let btn = ui.add_enabled(
                            !self.login.en_cours,
                            egui::Button::new(label_btn),
                        );
                        if btn.clicked() {
                            self.lancer_auth();
                        }

                        let toggle_label = if self.login.inscription {
                            "J'ai déjà un compte"
                        } else {
                            "Créer un compte"
                        };
                        if ui.button(toggle_label).clicked() {
                            self.login.inscription = !self.login.inscription;
                            self.login.message.clear();
                        }
                    });

                    if self.login.en_cours {
                        ui.add_space(8.0);
                        ui.spinner();
                    }
                });
        });

        if self.login.en_cours {
            ctx.request_repaint_after(std::time::Duration::from_millis(60));
        }
    }

    fn lancer_auth(&mut self) {
        if self.login.pseudo.trim().is_empty() || self.login.mot_de_passe.is_empty() {
            self.login.message = "Pseudo et mot de passe requis.".to_string();
            return;
        }

        self.login.en_cours = true;
        self.login.message.clear();

        let pseudo = self.login.pseudo.trim().to_string();
        let mot_de_passe = self.login.mot_de_passe.clone();
        let inscription = self.login.inscription;

        let (tx, rx) = mpsc::channel::<LoginResultat>();
        self.login.rx = Some(rx);

        std::thread::spawn(move || {
            // L'auth est volontairement faite hors du thread UI :
            // même si la DB répond lentement, l'interface reste fluide.
            let rt = match tokio::runtime::Runtime::new() {
                Ok(r) => r,
                Err(e) => {
                    let _ = tx.send(LoginResultat::Erreur(format!("Erreur runtime: {e}")));
                    return;
                }
            };

            rt.block_on(async move {
                dotenvy::dotenv().ok();
                let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
                    "postgres://poker:poker@localhost:5432/poker".to_string()
                });

                let pool = match sqlx::PgPool::connect(&url).await {
                    Ok(p) => p,
                    Err(e) => {
                        let _ = tx.send(LoginResultat::Erreur(format!("DB inaccessible: {e}")));
                        return;
                    }
                };

                let resultat = if inscription {
                    crate::db::joueur_repo::inscrire(&pool, &pseudo, &mot_de_passe).await
                        .map(|j| LoginResultat::Ok { pseudo: j.pseudo, jetons: j.jetons as u32, db_id: j.id })
                        .unwrap_or_else(LoginResultat::Erreur)
                } else {
                    match crate::db::joueur_repo::authentifier(&pool, &pseudo, &mot_de_passe).await {
                        Ok(Some(j)) => LoginResultat::Ok { pseudo: j.pseudo, jetons: j.jetons as u32, db_id: j.id },
                        Ok(None) => LoginResultat::Erreur("Pseudo ou mot de passe incorrect.".to_string()),
                        Err(e) => LoginResultat::Erreur(e),
                    }
                };

                let _ = tx.send(resultat);
            });
        });
    }
}
