use crate::games::slotmachine::SlotMachine;
use eframe::egui;
use rand::Rng;
use super::theme::{back_button, panel_frame, premium_button, GOLD_SOFT, TEXT_DIM};

pub(super) struct SlotMachineAnim {
    pub start_time: std::time::Instant,
    pub duration: std::time::Duration,
    pub final_symbols: [usize; 3],
    pub current_display: [usize; 3],
    pub fixed_count: usize,
}

impl super::CasinoApp {
    pub(super) fn ui_slot_machine(&mut self, ui: &mut egui::Ui) {
        if let Some(anim) = &mut self.slot_anim {
            let elapsed = anim.start_time.elapsed();
            if elapsed >= anim.duration {
                self.slot_symbols = anim.final_symbols;
                self.slot_anim = None;
            } else {
                let progress = elapsed.as_secs_f32() / anim.duration.as_secs_f32().max(0.001);
                let fixed = (progress * 3.0).floor() as usize;
                if fixed > anim.fixed_count {
                    anim.fixed_count = fixed.min(3);
                }
                let mut rng = rand::thread_rng();
                for i in anim.fixed_count..3 {
                    anim.current_display[i] = rng.gen_range(0..4);
                }
                for i in 0..anim.fixed_count.min(3) {
                    anim.current_display[i] = anim.final_symbols[i];
                }
                self.slot_symbols = anim.current_display;
                ui.ctx()
                    .request_repaint_after(std::time::Duration::from_millis(60));
            }
        }

        panel_frame().show(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.heading("Machine a sous");
                ui.label(egui::RichText::new("Rouleaux premium, style arcade-casino.").color(TEXT_DIM));
                ui.add_space(20.0);
                let highlight =
                    self.slot_symbols[0] == self.slot_symbols[1] && self.slot_symbols[1] == self.slot_symbols[2];
                dessiner_slot_machine(ui, &self.slot_symbols, highlight);
                ui.add_space(14.0);
                if premium_button(ui, "Lancer !").clicked()
                {
                    self.debiter_banque_joueur_avec_source(self.slot_mise, "Machine a sous - Mise");
                    let result = SlotMachine::spin();
                    self.slot_symbols = result.symbols;
                    
                    let mut rng = rand::thread_rng();
                    let duration = std::time::Duration::from_millis(rng.gen_range(1500..=2500));
                    self.slot_anim = Some(SlotMachineAnim {
                        start_time: std::time::Instant::now(),
                        duration,
                        final_symbols: result.symbols,
                        current_display: [0, 1, 2],
                        fixed_count: 0,
                    });
                    
                    // Stocker si c'est un gain
                    self.slot_result = if result.win {
                        let gain = self.slot_mise * 10;
                        self.crediter_banque_joueur_avec_source(gain, "Machine a sous - Gain");
                        format!("Jackpot ! (+{} €)", gain)
                    } else {
                        "Perdu...".to_string()
                    };
                }
                ui.add_space(10.0);
                if highlight {
                    ui.colored_label(GOLD_SOFT, &self.slot_result);
                } else {
                    ui.label(&self.slot_result);
                }
                if back_button(ui, "<- Retour menu").clicked() {
                    self.ecran = super::EcranCasino::Menu;
                }
            });
        });
    }
}

fn dessiner_slot_machine(ui: &mut egui::Ui, symbols: &[usize; 3], highlight: bool) {
    static SYMBOLS: [&str; 4] = ["🍒", "🍋", "🔔", "7"];
    let (rect, _response) = ui.allocate_exact_size(egui::vec2(400.0, 140.0), egui::Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 22.0, egui::Color32::from_rgb(49, 18, 23));
    painter.rect_stroke(
        rect,
        22.0,
        egui::Stroke::new(3.0, egui::Color32::from_rgb(219, 176, 76)),
        egui::StrokeKind::Outside,
    );

    for i in 0..3 {
        let x = rect.left() + 40.0 + i as f32 * 86.0;
        let y = rect.top() + 30.0;
        let slot_rect = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(60.0, 80.0));
        let color = if highlight {
            egui::Color32::from_rgb(255, 230, 122)
        } else {
            egui::Color32::from_rgb(241, 236, 222)
        };
        painter.rect_filled(slot_rect, 12.0, color);
        painter.rect_stroke(
            slot_rect,
            8.0,
            egui::Stroke::new(2.0, egui::Color32::GRAY),
            egui::StrokeKind::Outside,
        );
        painter.text(
            slot_rect.center(),
            egui::Align2::CENTER_CENTER,
            SYMBOLS[symbols[i]],
            egui::FontId::proportional(48.0),
            egui::Color32::BLACK,
        );
    }

    let levier_x = rect.right() - 30.0;
    let levier_y = rect.center().y;
    let levier_top = egui::pos2(levier_x, levier_y - 30.0);
    let levier_bottom = egui::pos2(levier_x, levier_y + 30.0);
    painter.line_segment(
        [levier_top, levier_bottom],
        egui::Stroke::new(6.0, egui::Color32::DARK_GRAY),
    );
    painter.circle_filled(levier_top, 10.0, egui::Color32::from_rgb(220, 0, 0));
    painter.circle_filled(levier_bottom, 12.0, egui::Color32::from_rgb(180, 180, 180));
}
