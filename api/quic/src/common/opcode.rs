bitflags::bitflags! {
    pub struct Opcode: u8 {
        const ARP = 0b10000000;
        const TEXT = 0b00000001;
    }
}

impl Default for Opcode {
    fn default() -> Self {
        Self::TEXT
    }
}
