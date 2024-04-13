pub const LIGHT_STATE_LEN: usize = 8;

#[derive(Copy, Clone, Debug)]
pub struct LightState {
    pub cold: u16,
    pub warm: u16,
    x: u16,
    y: u16,
}

impl Default for LightState {
    fn default() -> Self {
        LightState {
            cold: 16000,
            warm: 16000,
            x: 197,
            y: 164,
        }
    }
}

impl LightState {
    pub fn from_bytes(bytes: &[u8; LIGHT_STATE_LEN]) -> Self {
        Self {
            cold: u16::from_le_bytes([bytes[0], bytes[1]]),
            warm: u16::from_le_bytes([bytes[2], bytes[3]]),
            x: u16::from_le_bytes([bytes[4], bytes[5]]),
            y: u16::from_le_bytes([bytes[6], bytes[7]]),
        }
    }

    pub fn into_bytes(self) -> [u8; LIGHT_STATE_LEN] {
        let [c0, c1] = self.cold.to_le_bytes();
        let [w0, w1] = self.warm.to_le_bytes();
        let [x0, x1] = self.x.to_le_bytes();
        let [y0, y1] = self.y.to_le_bytes();
        [c0, c1, w0, w1, x0, x1, y0, y1]
    }
}
