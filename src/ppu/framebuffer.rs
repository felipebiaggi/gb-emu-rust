const WIDHT: usize = 160;
const HEIGHT: usize = 144;

pub struct FrameBuffer {
    pub pixels: [u8; WIDHT * HEIGHT],
}

impl FrameBuffer {
    pub fn new() -> Self {
        Self {
            pixels: [0; WIDHT * HEIGHT],
        }
    }

    pub fn set(&mut self, x: usize, y: usize, value: u8) {
        self.pixels[y * WIDHT + x] = value & 0b11;
    }

    pub fn get(&mut self, x: usize, y: usize) -> u8 {
        self.pixels[y * WIDHT + x] & 0b11
    }

    pub fn clear(&mut self, value: u8) {
        let c = value & 0b11;
        self.pixels.fill(c);
    }
}
