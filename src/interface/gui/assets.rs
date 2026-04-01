use eframe::egui;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum GameAsset {
    Wallpaper,
    Poker,
    Blackjack,
    Slot,
    HiLo,
    Roulette,
    Diamond,
    Mines,
    Crash,
    Plane,
}

impl GameAsset {
    pub(super) fn image(self) -> egui::ImageSource<'static> {
        match self {
            GameAsset::Wallpaper => egui::include_image!("../../../assets/walpaper.png"),
            GameAsset::Poker => egui::include_image!("../../../assets/Poker-Texas.png"),
            GameAsset::Blackjack => egui::include_image!("../../../assets/Blackjack.png"),
            GameAsset::Slot => egui::include_image!("../../../assets/Slot.png"),
            GameAsset::HiLo => egui::include_image!("../../../assets/Hi-Lo.png"),
            GameAsset::Roulette => egui::include_image!("../../../assets/Roulette.png"),
            GameAsset::Diamond => egui::include_image!("../../../assets/diamond.png"),
            GameAsset::Mines => egui::include_image!("../../../assets/Mines1.png"),
            GameAsset::Crash => egui::include_image!("../../../assets/Crash1.png"),
            GameAsset::Plane => egui::include_image!("../../../assets/avion.png"),
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
