pub struct SlotMachineResult {
    pub symbols: [usize; 3],
    pub win: bool,
}

pub struct SlotMachine;

impl SlotMachine {
    pub fn spin() -> SlotMachineResult {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let symbols = [
            rng.gen_range(0..4),
            rng.gen_range(0..4),
            rng.gen_range(0..4),
        ];
        let win = symbols[0] == symbols[1] && symbols[1] == symbols[2];
        SlotMachineResult { symbols, win }
    }
}
