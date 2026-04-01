/// Type de mise possible à la roulette (doit matcher l'UI)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RouletteBet {
    // Chaque variante correspond à un vrai type de pari de roulette européenne.
    Number(u8), // Plein
    Color(RouletteColor),
    Column(u8), // 0,1,2 (1ère, 2ème, 3ème colonne)
    Dozen(u8),  // 0,1,2 (1st 12, 2nd 12, 3rd 12)
    Even,
    Odd,
    Low,   // 1-18
    High,  // 19-36
    None,
}

/// Calcule le multiplicateur de gain pour une mise donnée et un résultat
pub fn gain_multiplier(bet: RouletteBet, result: &RouletteResult) -> u32 {
    // On renvoie ici le gain net, pas le remboursement total.
    // Le total final est recalculé ensuite à partir de la mise.
    match bet {
        RouletteBet::None => 0,
        RouletteBet::Number(n) => {
            if n == result.number { 35 } else { 0 }
        }
        RouletteBet::Color(c) => {
            if c == result.color && c != RouletteColor::Green { 1 } else { 0 }
        }
        RouletteBet::Column(col) => {
            // Colonne : (n-1)%3 == col
            // Gain net 2x la mise (total 3x la mise, comme à la roulette européenne)
            if result.number >= 1 && result.number <= 36 && (result.number - 1) % 3 == col { 2 } else { 0 }
        }
        RouletteBet::Dozen(dz) => {
            // Douzaine : 0=1-12, 1=13-24, 2=25-36
            // Gain net 2x la mise (total 3x la mise, comme à la roulette européenne)
            let n = result.number;
            if n >= 1 && n <= 12 && dz == 0 { 2 }
            else if n >= 13 && n <= 24 && dz == 1 { 2 }
            else if n >= 25 && n <= 36 && dz == 2 { 2 }
            else { 0 }
        }
        RouletteBet::Even => {
            if result.number >= 1 && result.number <= 36 && result.number % 2 == 0 { 1 } else { 0 }
        }
        RouletteBet::Odd => {
            if result.number >= 1 && result.number <= 36 && result.number % 2 == 1 { 1 } else { 0 }
        }
        RouletteBet::Low => {
            if result.number >= 1 && result.number <= 18 { 1 } else { 0 }
        }
        RouletteBet::High => {
            if result.number >= 19 && result.number <= 36 { 1 } else { 0 }
        }
    }
}
pub struct RouletteResult {
    // On garde à la fois le numéro tiré et le résultat économique du pari.
    // Ça évite de refaire les calculs ailleurs dans le code.
    pub number: u8, // 0-36
    pub color: RouletteColor,
    pub win: bool,
    pub gain_net: u32,  // Multiplicateur net (0, 1, 2, 35)
    pub total_payout: u32,  // Total reçu au joueur (0 si perte, mise*(1+gain_net) si gain)
}

/// Ordre réel des numéros sur une roue européenne (sens horaire)
pub const EUROPEAN_WHEEL_ORDER: [u8; 37] = [
    0, 32, 15, 19, 4, 21, 2, 25, 17, 34, 6, 27, 13, 36, 11, 30, 8, 23, 10, 5, 24, 16, 33, 1, 20, 14, 31, 9, 22, 18, 29, 7, 28, 12, 35, 3, 26
];

/// Retourne la couleur réelle d'un numéro sur la roue européenne (0 = vert, puis alternance rouge/noir)
pub fn european_color_for_number(number: u8) -> RouletteColor {
    // On ne calcule pas la couleur avec une simple parité :
    // sur une vraie roue, l'ordre des couleurs dépend de la position réelle des numéros.
    if number == 0 {
        RouletteColor::Green
    } else if let Some(idx) = EUROPEAN_WHEEL_ORDER.iter().position(|&n| n == number) {
        if idx % 2 == 1 {
            RouletteColor::Red
        } else {
            RouletteColor::Black
        }
    } else {
        RouletteColor::Green // fallback
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RouletteColor {
    Red,
    Black,
    Green,
}

pub struct Roulette;

impl Roulette {
    pub fn spin() -> RouletteResult {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        // On tire d'abord un index sur la vraie roue, puis on en déduit le numéro et la couleur.
        let idx = rng.gen_range(0..=36);
        let number = EUROPEAN_WHEEL_ORDER[idx];
        let color = european_color_for_number(number);
        RouletteResult { number, color, win: false, gain_net: 0, total_payout: 0 }
    }
}
