use crate::games::slotmachine::SlotMachine;
use eframe::egui;
use super::theme::{back_button, panel_frame, premium_button, GOLD_SOFT, TEXT_DIM};

impl super::CasinoApp {
    pub(super) fn ui_slot_machine(&mut self, ui: &mut egui::Ui) {
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
                    let result = SlotMachine::spin();
                    self.slot_symbols = result.symbols;
                    self.slot_result = if result.win {
                        "Jackpot !".to_string()
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
