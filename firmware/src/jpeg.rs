use stm32ral::{jpeg, write_reg, read_reg};

#[derive(Copy, Clone, Debug)]
pub enum Error {
    WrongResolution,
    WrongColourspace,
    WrongSubsampling,
    BadHeader,
    ConversionIncomplete,
    OutputTooSmall,
}

pub type Result<T> = core::result::Result<T, Error>;

/// JPEG decoder.
pub struct Jpeg {
    jpeg: jpeg::Instance,
}

impl Jpeg {
    pub fn new(jpeg: jpeg::Instance) -> Self {
        Self { jpeg }
    }

    /// Decode the provided JPEG data into the output buffer,
    /// which must be large enough to contain all output pixels,
    /// for a 64x64 image that's 3072 u32.
    pub fn decode(&self, data: &[u8], out: &mut [u32]) -> Result<()> {
        // Ensure JPEG core is enabled and flush input and output.
        write_reg!(jpeg, self.jpeg, CR, JCEN: 1, OFF: 1, IFF: 1);

        // Enable header processing and decoder mode.
        write_reg!(jpeg, self.jpeg, CONFR1, HDR: 1, DE: 1);

        // Clear any leftover flags.
        write_reg!(jpeg, self.jpeg, CFR, CEOCF: 1, CHPDF: 1);

        // Begin image decoding.
        write_reg!(jpeg, self.jpeg, CONFR0, START: 0);
        write_reg!(jpeg, self.jpeg, CONFR0, START: 1);

        // Write any initial unaligned bytes.
        let off = data.as_ptr().align_offset(4);
        match off {
            1 => write_reg!(jpeg, self.jpeg, DIR,
                            u32::from_le_bytes([0, 0, 0, data[0]])),
            2 => write_reg!(jpeg, self.jpeg, DIR,
                            u32::from_le_bytes([0, 0, data[0], data[1]])),
            3 => write_reg!(jpeg, self.jpeg, DIR,
                            u32::from_le_bytes([0, data[0], data[1], data[2]])),
            _ => (),
        }

        // Convert JPEG slice of u8 to a slice of u32, since JPEG peripheral needs to be fed
        // u32 data and it's inefficient to create each input u32 in turn. In future, use MDMA
        // to perform data repacking and everything else.
        // NOTE(unsafe): We've manually aligned and the underlying memory is already accessible
        // NOTE(unsafe): through the original `jpeg` slice.
        let rem = &data[off..];
        let data32: &[u32] = unsafe {
            core::slice::from_raw_parts(rem.as_ptr() as *const u32, rem.len()/4)
        };

        // Store position to write new output words to.
        let mut outidx = 0;

        let mut got_header = false;

        for word in data32.iter() {
            // Check for finishing parsing header data.
            if read_reg!(jpeg, self.jpeg, SR, HPDF == 1) {
                write_reg!(jpeg, self.jpeg, CFR, CHPDF: 1);
                // Confirm image parameters match our requirements exactly:
                // Must be 64x64 YUV with 4:4:4 chroma.
                let (ysize, cs, nf) = read_reg!(jpeg, self.jpeg, CONFR1, YSIZE, COLORSPACE, NF);
                let xsize = read_reg!(jpeg, self.jpeg, CONFR3, XSIZE);
                let (hsf1, vsf1) = read_reg!(jpeg, self.jpeg, CONFRN1, HSF, VSF);
                let (hsf2, vsf2) = read_reg!(jpeg, self.jpeg, CONFRN2, HSF, VSF);
                let (hsf3, vsf3) = read_reg!(jpeg, self.jpeg, CONFRN3, HSF, VSF);
                if xsize != 64 || ysize != 64 {
                    return Err(Error::WrongResolution);
                }
                if cs != 1 || nf != 2 {
                    return Err(Error::WrongColourspace);
                }
                if (hsf1, vsf1, hsf2, vsf2, hsf3, vsf3) != (1, 1, 1, 1, 1, 1) {
                    return Err(Error::WrongSubsampling);
                }
                got_header = true;
            }

            // Read any processed data.
            while read_reg!(jpeg, self.jpeg, SR, OFNEF == 1) {
                if outidx >= out.len() {
                    return Err(Error::OutputTooSmall);
                }
                out[outidx] = read_reg!(jpeg, self.jpeg, DOR);
                outidx += 1;
            }

            // Wait for FIFO to have space for more data, then feed it in.
            while read_reg!(jpeg, self.jpeg, SR, IFNFF == 0) {}
            write_reg!(jpeg, self.jpeg, DIR, *word);
        }

        // Return an error if we didn't find the header by the time we finished writing the
        // input data, as it suggests the input was not a valid JPEG and in any event cannot
        // be processed.
        if !got_header {
            return Err(Error::BadHeader);
        }

        // Feed in any remaining unaligned data.
        while read_reg!(jpeg, self.jpeg, SR, IFNFF == 0) {}
        let rem = rem.chunks_exact(4).remainder();
        match rem.len() {
            1 => write_reg!(jpeg, self.jpeg, DIR,
                            u32::from_le_bytes([rem[0], 0, 0, 0])),
            2 => write_reg!(jpeg, self.jpeg, DIR,
                            u32::from_le_bytes([rem[0], rem[1], 0, 0])),
            3 => write_reg!(jpeg, self.jpeg, DIR,
                            u32::from_le_bytes([rem[0], rem[1], rem[2], 0])),
            _ => (),
        }

        // Finish reading remaining output data once input processing is done.
        while read_reg!(jpeg, self.jpeg, SR, OFNEF == 1) {
            if outidx >= out.len() {
                return Err(Error::OutputTooSmall);
            }
            out[outidx] = read_reg!(jpeg, self.jpeg, DOR);
            outidx += 1;
        }

        // Check for end of conversion.
        if read_reg!(jpeg, self.jpeg, SR, EOCF != 1) {
            return Err(Error::ConversionIncomplete);
        }

        Ok(())
    }
}
