use eframe::egui;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum GameAsset {
    Poker,
    Blackjack,
    Slot,
    HiLo,
}

impl GameAsset {
    pub(super) fn image(self) -> egui::ImageSource<'static> {
        match self {
            GameAsset::Poker => egui::include_image!("../../../assets/Poker-Texas.png"),
            GameAsset::Blackjack => egui::include_image!("../../../assets/Blackjack.png"),
            GameAsset::Slot => egui::include_image!("../../../assets/Slot.png"),
            GameAsset::HiLo => egui::include_image!("../../../assets/Hi-Lo.png"),
        }
    }

    pub(super) fn native_size(self) -> egui::Vec2 {
        egui::vec2(1536.0, 1024.0)
    }
}

pub(super) fn paint_contained_art(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    asset: GameAsset,
    radius: u8,
) {
    let source = asset.native_size();
    let scale = (rect.width() / source.x).min(rect.height() / source.y);
    let size = egui::vec2(source.x * scale, source.y * scale);
    let target = egui::Rect::from_center_size(rect.center(), size);

    ui.put(
        target,
        egui::Image::new(asset.image())
            .corner_radius(egui::CornerRadius::same(radius))
            .fit_to_exact_size(size),
    );
}
