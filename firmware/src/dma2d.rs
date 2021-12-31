use stm32ral::{dma2d, write_reg, read_reg};
use crate::framebuf::MainFrameBuf;

#[derive(Copy, Clone, Debug)]
pub enum Error {
    NotEnoughData,
    InternalCE,
    InternalTE,
}

pub type Result<T> = core::result::Result<T, Error>;

/// Driver for the DMA2D peripheral.
pub struct DMA2D {
    dma2d: dma2d::Instance,
}

impl DMA2D {
    /// Create a new DMA2D driver.
    pub fn new(dma2d: dma2d::Instance) -> Self {
        Self { dma2d }
    }

    /// Convert YUV-coded JPEG MCUs from the JPEG peripheral
    /// into RGB888 pixel data in the output framebuffer.
    ///
    /// The JPEG must use 4:4:4 chroma subsampling in the YUV colourspace,
    /// with a resolution of exactly 64x64 pixels, and so the input data
    /// must be 3072 words long.
    pub fn convert_jpeg(&self, data: &[u32], out: &mut MainFrameBuf) -> Result<()> {
        if data.len() < 3072 {
            return Err(Error::NotEnoughData);
        }

        // NOTE(unsafe): DMA operation will finish before we return, staying within lifetime.
        unsafe { write_reg!(dma2d, self.dma2d, FGMAR, data.as_ptr() as u32) };
        unsafe { write_reg!(dma2d, self.dma2d, OMAR, out.0.as_ptr() as u32) };
        write_reg!(dma2d, self.dma2d, FGOR, 0);
        write_reg!(dma2d, self.dma2d, OOR, 0);
        write_reg!(dma2d, self.dma2d, FGPFCCR, CSS: 0, CM: 0b1011);
        write_reg!(dma2d, self.dma2d, OPFCCR, RBS: 1, CM: RGB888);
        write_reg!(dma2d, self.dma2d, NLR, PL: 64, NL: 64);
        write_reg!(dma2d, self.dma2d, IFCR, 0x3f);
        write_reg!(dma2d, self.dma2d, CR, MODE: MemoryToMemoryPFC, START: Start);

        loop {
            let (ceif, tcif, teif) = read_reg!(dma2d, self.dma2d, ISR, CEIF, TCIF, TEIF);
            if ceif == 1 {
                return Err(Error::InternalCE);
            }
            if teif == 1 {
                return Err(Error::InternalTE);
            }
            if tcif == 1 {
                break;
            }
        }

        Ok(())
    }
}
