//! Framebuffer that implements embedded_graphic's DrawTarget.

use embedded_graphics::{
    Pixel,
    pixelcolor::{Rgb888, RgbColor},
    draw_target::DrawTarget,
    geometry::{OriginDimensions, Size},
};

#[repr(transparent)]
#[derive(Copy, Clone)]
pub struct FrameBuf<const X: usize, const Y: usize>(pub [[[u8; 3]; X]; Y]);

pub type MainFrameBuf = FrameBuf<64, 64>;
pub type SubFrameBuf = FrameBuf<160, 80>;

impl <const X: usize, const Y: usize> FrameBuf<X, Y> {
    /// Set all pixels to black.
    pub fn clear_black(&mut self) {
        for x in 0..X {
            for y in 0..Y {
                self.0[y][x] = [0, 0, 0];
            }
        }
    }

    /// Return a slice that aliases the same memory.
    pub fn as_slice(&self) -> &[u8] {
        // NOTE(unsafe): Creates a shared reference to the same underlying data,
        // NOTE(unsafe): which we know is tightly packed and so a valid [u8].
        unsafe { core::slice::from_raw_parts(self as *const _ as *const u8,
                                             core::mem::size_of::<Self>()) }
    }
}

impl <const X: usize, const Y: usize> OriginDimensions for FrameBuf<X, Y> {
    fn size(&self) -> Size {
        Size::new(X as u32, Y as u32)
    }
}

impl <const X: usize, const Y: usize> DrawTarget for FrameBuf<X, Y> {
    type Color = Rgb888;
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
        where I: IntoIterator<Item = Pixel<Self::Color>>
    {
        for Pixel(coord, color) in pixels.into_iter() {
            if let Ok(pos) = coord.try_into() {
                let (x, y): (u32, u32) = pos;
                self.0[y as usize][x as usize] = [color.r(), color.g(), color.b()];
            }
        }
        Ok(())
    }

    fn clear(&mut self, color: Self::Color) -> Result<(), Self::Error> {
        let color = [color.r(), color.g(), color.b()];
        for x in 0..X {
            for y in 0..Y {
                self.0[y][x] = color;
            }
        }
        Ok(())
    }
}
