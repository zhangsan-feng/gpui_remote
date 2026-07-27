use gpui::{Rgba, rgb};

pub fn rgb_to_u32(r: u8, g: u8, b: u8) -> Rgba {
    rgb((r as u32) << 16 | (g as u32) << 8 | b as u32)
}
