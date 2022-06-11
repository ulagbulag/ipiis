ipis::bitflags::bitflags! {
    pub struct Result: u8 {
        const ACK = 0b10000000;
        const OK = 0b01000000;
        const ERR = 0b00100000;

        const ACK_OK = Self::ACK.bits | Self::OK.bits;
        const ACK_ERR = Self::ACK.bits | Self::ERR.bits;
    }
}
