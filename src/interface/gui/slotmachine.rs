use crate::games::slotmachine::SlotMachine;
use eframe::egui;

impl super::CasinoApp {
    pub(super) fn ui_slot_machine(&mut self, ui: &mut egui::Ui) {
        ui.add_space(40.0);
        ui.vertical_centered(|ui| {
            ui.heading("Machine a sous");
            ui.add_space(20.0);
            let highlight =
                self.slot_symbols[0] == self.slot_symbols[1] && self.slot_symbols[1] == self.slot_symbols[2];
            dessiner_slot_machine(ui, &self.slot_symbols, highlight);
            ui.add_space(10.0);
            
            ui.horizontal(|ui| {
                ui.label("Ta mise :");
                let max_mise = self.banque_joueur.max(1);
                if self.slot_mise > max_mise {
                    self.slot_mise = max_mise;
                }
                ui.add(egui::Slider::new(&mut self.slot_mise, 1..=max_mise).text("€"));
            });
            ui.label(format!("Jackpot potentiel : {} €", self.slot_mise * 10));
            
            ui.horizontal(|ui| {
                ui.add_space(750.0);
                if self.banque_joueur < self.slot_mise {
                    ui.colored_label(egui::Color32::RED, "Pas assez d'euros !");
                } else if ui
                    .add(egui::Button::new("Lancer !").min_size(egui::vec2(100.0, 40.0)))
                    .clicked()
                {
                    self.banque_joueur -= self.slot_mise;
                    let result = SlotMachine::spin();
                    self.slot_symbols = result.symbols;
                    self.slot_result = if result.win {
                        let gain = self.slot_mise * 10;
                        self.banque_joueur += gain;
                        format!("Jackpot ! (+{} €)", gain)
                    } else {
                        "Perdu...".to_string()
                    };
                }
            });
            ui.add_space(10.0);
            if highlight {
                ui.colored_label(egui::Color32::from_rgb(255, 215, 0), &self.slot_result);
            } else {
                ui.label(&self.slot_result);
            }
            if ui.button("<- Retour menu").clicked() {
                self.ecran = super::EcranCasino::Menu;
            }
        });
    }
}

fn dessiner_slot_machine(ui: &mut egui::Ui, symbols: &[usize; 3], highlight: bool) {
    static SYMBOLS: [&str; 4] = ["🍒", "🍋", "🔔", "7"];
    let (rect, _response) = ui.allocate_exact_size(egui::vec2(400.0, 140.0), egui::Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 16.0, egui::Color32::from_rgb(40, 40, 40));
    painter.rect_stroke(
        rect,
        16.0,
        egui::Stroke::new(3.0, egui::Color32::from_rgb(200, 180, 60)),
        egui::StrokeKind::Outside,
    );

    for i in 0..3 {
        let x = rect.left() + 40.0 + i as f32 * 86.0;
        let y = rect.top() + 30.0;
        let slot_rect = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(60.0, 80.0));
        let color = if highlight {
            egui::Color32::from_rgb(255, 220, 80)
        } else {
            egui::Color32::from_rgb(230, 230, 230)
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
