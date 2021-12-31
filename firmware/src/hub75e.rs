//! HUB75E driver for STM32H743.
//!
//! # HUB75E concept of operation
//!
//! The HUB75E interface has five row-select ("address") pins A-E,
//! six data pins (R1, G1, B1, R2, G2, B2), and clock, latch, and output-enable pins.
//! The 32 possible rows addressed by the five address pins each correspond to
//! two physical rows of LEDs: the selected row and the one 32 rows down.
//!
//! To pulse some LEDs on a given row on or off, we select that row with the
//! address pins, then shift out two rows worth of colour data on the RGB pins
//! (for the selected row and the row 32 down), pulsing the latch pin on the final
//! data word. Then, we pulse the OE pin low for the required duration.
//!
//! To control the LED brightness beyond just on/off, we compute and send ten
//! different lines of data for each row, and double the pulse width each time.
//! By selecting which LEDs are on and off in each phase, we obtain 1024 levels
//! of on-time for each LED in the row, giving 10-bit resolution per colour.
//! This process is known as binary code modulation (BCM).
//!
//! Because the eye's intensity response is non-linear, we map the framebuffer
//! data which contains 8 bits per colour through a gamma lookup table to obtain
//! the 10 bits per colour display resolution, in the process scaling the intensity
//! to give better linear fading and colour matching.
//!
//! # Driver concept of operation
//!
//! The six data pins and the latch pin are connected to pins 0-6 of the same
//! GPIO port and configured as outputs, and pin 7 must not be configured as
//! an output. It is then possible to update all seven pins at once by writing
//! a single byte to the ODR register of that GPIO port, while not disturbing
//! pin 7 (which may still be used as an input or alternate function).
//!
//! The clock pin is driven by a timer which generates the pixel clock and
//! triggers a DMA request on each rising edge. This DMA request prompts
//! the DMA peripheral to read the next byte of data from the line buffer
//! and write it to the ODR register, setting the data pins. The line buffer
//! is computed so that the latch pin is pulsed on the final data byte.
//! Because the HUB75E interface reads data on the clock rising edge, the
//! DMA must take at least the hold time (5ns) to update the data pins.
//! By triggering the DMA on the rising edge, we give the DMA the maximum
//! possible time to finish updating before the setup phase (also 5ns) begins.
//!
//! When the entire line buffer is written out by the DMA, a DMA interrupt
//! fires, which disables the pixel clock and activates the output enable pin.
//!
//! The output enable pin is driven by a second timer which is programmed
//! to generate one-shot active-low pulses at the different widths required
//! for the BCM modulation. It is started by the DMA interrupt and then
//! generates an interrupt when the pulse has finished, which begins the
//! next DMA transfer.
//!
//! Finally, the address pins are just ordinary GPIO outputs which are
//! written to the current line immediately before starting each DMA transfer.
//!
//! # Driver operations sequence
//!
//! 1. Call [`Hub75E::start()`]
//!     * Sets `line=0` and `bcm=0`
//!     * Caches first row of gamma-mapped framebuffer data into `gbuf`
//!     * Computes first line of BCM data into `lbuf`
//!     * Starts first DMA transfer and pixel clock, swaps `lbuf` to second buffer
//!     * Increments to `bcm=1`
//!     * Computes second line of BCM data into new `lbuf`
//! 2. DMA complete interrupt fires, call [`Hub75E::dma_isr()`]
//!     * OE timer activated with required pulse width for BCM phase 0
//!     * Pixel clock stopped
//! 3. OE timer interrupt fires, call [`Hub75E::tim_oe_isr()`]
//!     * Starts next DMA transfer and pixel clock, swaps `lbuf` back to first buffer
//!     * Increments to `bcm=2`
//!     * Computes third line of BCM data into new `lbuf` while DMA transfer is ongoing
//! 4. DMA complete interrupt fires, call [`Hub75E::dma_isr()`]
//!     * OE timer activated with required pulse width
//!     * Pixel clock stopped
//!     * If we just finished the DMA transfer for the final BCM phase of this line,
//!       we now increment `line`, cache the next row of gamma-mapped framebuffer data,
//!       set `bcm` back to 0 and compute the first line of its BCM data.
//!       * Normally this is done in the previous OE ISR, but after the final BCM phase
//!         it takes much longer to load the next framebuffer line, and the final OE pulse
//!         is also very long, so it's more efficient to compute this now, during the OE pulse.
//! 5. OE timer interrupt fires, call [`Hub75E::tim_oe_isr()`]
//!     * Starts next DMA transfer and pixel clock, swaps `lbuf` to other buffer
//!     * Normally, increment `bcm` and compute next line of BCM data into new `lbuf`
//!       while DMA transfer is ongoing
//!     * However, if we just finished the OE pulse for the final BCM phase of this line,
//!       we'll skip computing the next line because it takes longer to map the framebuffer
//!       data than the DMA transfer takes, and because the next OE pulse will be much longer.
//!       * Still set `bcm` to 0 in this case to signal to the DMA ISR that it should
//!         perform the computation instead.
//! 6. Repeat from 4.
//!
//! # Notes on timing
//!
//! For the STM32H743 with CPU at 240MHz and PCLK at 60MHz, it takes about 25µs to gamma-map
//! one row of framebuffer data, and about 5µs to compute each BCM phase. At 15MHz pixel clock
//! (which is about as high as it can go at 60MHz PCLK before the DMA can't keep up), it also
//! takes about 5µs to shift out 65 data words, hence computing each BCM phase during each
//! DMA write operation while saving the gamma mapping for the final OE pulse.
//!
//! For ten-level BCM, starting with a 125ns pulse and finishing at 64µs seems to work well.
//!
//! In this configuration the maximum LED duty cycle is 71% (excluding the 1:32 muxing),
//! in other words the maximum panel brightness is 71% of theoretical. CPU usage is about
//! 40% in the TIM and DMA ISRs, and a framerate of 175fps is achieved.
//!
//! If all clocks are doubled (CPU to 480MHz, PCLK to 120MHz), a 30MHz pixel clock is possible,
//! computation times halve, giving 204 fps, LED duty cycle of 83%, and CPU usage drops to 23%.
//!
//! It would also be straightforward to increase to 11-level gamma mapping, either using a shorter
//! 62.5ns initial pulse (at the cost of lower duty cycle) or a longer 128µs final pulse (improving
//! duty cycle but reducing frame rate). However it's not clear there's much noticable benefit.

use crate::{gpio::Hub, tim::Tim, dma::DMAStream, framebuf::MainFrameBuf, LineBuf};

/// Driver for HUB75E LED matrices.
///
/// Refer to the [module-level documentation](`crate::hub75e`) for more details.
pub struct Hub75E {
    /// GPIO controls for setting address lines and getting ODR address for DMA.
    io: Hub,
    /// Timer configured to output pixel clock and trigger DMA requests on rising edges.
    tim_clk: Tim,
    /// Timer configured for one-shot active-low nOE pulse generation.
    tim_oe: Tim,
    /// DMA stream, mapped to `tim_clk`'s DRQs.
    dma_stream: DMAStream,
    /// RGB888 frame buffer to render from. Can be swapped out at runtime.
    fbuf: &'static MainFrameBuf,
    /// Memory to render lines to, which must be accessible by DMA.
    lbufs: &'static mut [LineBuf; 2],
    /// Smallest pulse length in timer ticks for BCM.
    bcm_base: u32,
    /// Buffer the gamma lookup for the current double-line internally.
    gbuf: [u16; 384],
    /// Track current line buffer, 0..2.
    lbuf: u8,
    /// Track current double-line, 0..32.
    line: u8,
    /// Track current BCM phase, 0..10.
    bcm: u8,
    /// Number of BCM phases to skip, reducing output brightness.
    bcm_skip: u8,
}

impl Hub75E {
    const BCM_PHASES: u8 = 10;

    /// Create a new HUB75E driver.
    ///
    /// * `io`: [`crate::gpio::Hub`] instance which provides address setting and
    ///   the ODR address for DMA to write RGB data and control.
    /// * `tim_clk`: [`crate::tim::Tim`] instance configured to output a pixel clock
    ///    and DMA requests on each falling edge.
    /// * `tim_oe`: [`crate::tim::Tim`] instance configured for one-pulse generation
    ///    and interrupt requests after each pulse.
    /// * `dma_stream`: [`crate::dma::DMAStream`] instance.
    /// * `fbuf`: reference to 64x64 RGB888 framebuf to render.
    /// * `lbuf`: reference to 65-byte scratch buffer which must be readable by
    ///    the DMA peripheral.
    /// * `bcm_base`: Base number of cycles for least significant bit in BCM.
    pub fn new(
        io: Hub,
        tim_clk: Tim,
        tim_oe: Tim,
        dma_stream: DMAStream,
        fbuf: &'static MainFrameBuf,
        lbufs: &'static mut [LineBuf; 2],
        bcm_base: u32,
    ) -> Self {
        Self {
            io, tim_clk, tim_oe, dma_stream, fbuf, lbufs, bcm_base,
            gbuf: [0; 384], lbuf: 0, line: 0, bcm: 0, bcm_skip: 0,
        }
    }

    /// Call to begin rendering the framebuffer to the display.
    pub fn start(&mut self) {
        self.line = 0;
        self.bcm = 0;
        self.lbuf = 0;
        self.load_line();
        self.render_line();
        self.start_dma();
        self.process_next_line();
    }

    /// Call from the DMA ISR for the provided DMA peripheral.
    pub fn dma_isr(&mut self) {
        self.dma_stream.clear_tcif();

        // Trigger OE pulse.
        // Because we advance `self.bcm` immediately after starting the DMA,
        // the current value reflects the _next_ pulse width, so compute the
        // previous `self.bcm` value to use for the OE pulse width here.
        let bcm = if self.bcm == 0 { Self::BCM_PHASES - 1 } else { self.bcm - 1 };
        self.tim_oe.start_oneshot(self.bcm_base << bcm);

        // Stop pixel clock.
        self.tim_clk.stop();

        // Normally we compute the next BCM phase as soon as the OE pulse finishes,
        // in other words while the current phase data is still being clocked out.
        //
        // Since the computation takes about as long as the DMA write, this is
        // more efficient than doing it after the DMA write and during the OE pulse,
        // especially because most of the OE pulses are significantly shorter than
        // the computation time.
        //
        // However, computing the first BCM phase takes much longer as we must
        // perform the gamma mapping from the framebuffer, and simultaneously
        // the final OE pulse is much longer, so instead it makes sense to wait
        // for the DMA to complete and perform the first BCM phase computation
        // during the OE period.
        //
        // The timer ISR sets `self.bcm` to 0 without performing the processing,
        // so we revert bcm to BCM_PHASES-1 and then run the processing in this DMA ISR.
        if self.bcm == 0 {
            self.bcm = Self::BCM_PHASES - 1;
            self.process_next_line();
        }
    }

    /// Call from the `tim_oe` ISR.
    pub fn tim_oe_isr(&mut self) {
        // Clear interrupt flag.
        self.tim_oe.clear_uif();

        // Trigger next DMA write.
        self.start_dma();

        // Prepare for the next DMA transfer.
        //
        // We perform the computation for the next BCM phase in this timer ISR
        // (during the DMA write) except for the very first lookup of each line,
        // which takes much longer and it makes more sense to do this in the later
        // DMA ISR (during the OE pulse).
        //
        // In that case, set `self.bcm` to 0 (which is what `process_next_line()`
        // would normally do) but do not call `process_next_line()`, leaving it
        // for the DMA ISR to call.
        if self.bcm < Self::BCM_PHASES - 1 {
            self.process_next_line();
        } else {
            self.bcm = 0;
        }
    }

    /// Set a new framebuf.
    ///
    /// Note that this method is not synchronised to vbuf so at high framebuf
    /// update rates some tearing may be visible.
    pub fn set_fbuf(&mut self, fbuf: &'static MainFrameBuf) {
        self.fbuf = fbuf;
    }

    /// Change the number of BCM phases skipped on each line.
    ///
    /// This adjusts the overall brightness, from full brightness at 0
    /// to completely off when equal to Self::BCM_PHASES, with approximately
    /// linear brightness steps in between.
    pub fn set_bcm_skip(&mut self, bcm_skip: u8) {
        self.bcm_skip = bcm_skip;
    }

    /// Start writing the contents of LBUF to the HUB75E interface.
    ///
    /// Sets the current `self.line` to the address pins, sets up the DMA
    /// transfer, and then enables the pixel clock which drives the DMA.
    fn start_dma(&mut self) {
        // Set address decoders to current line.
        self.io.set_addr(self.line);

        // Start DMA engine.
        // Usually there's already a pending DRQ from the timer because it's
        // turned off _after_ the previous DMA transfer completes, so the DMA
        // engine writes the first byte immediately. If there isn't a pending
        // DRQ, we just get one dummy initial clock cycle first.
        self.dma_stream.start_tx(&self.lbufs[self.lbuf as usize]);

        // Start pixel clock, beginning DMA triggers.
        self.tim_clk.start();

        // Swap line buffer around so next operation writes to the unused one.
        self.lbuf ^= 1;
    }

    /// Compute required buffers for the next DMA transfer.
    ///
    /// Advances `self.bcm` and `self.line` as required, loads
    /// new gamma-mapped pixel data into `gbuf` on line change,
    /// and then computes the next BCM phase data to write.
    fn process_next_line(&mut self) {
        // Advance to next line or BCM phase.
        self.bcm += 1;
        if self.bcm == Self::BCM_PHASES {
            self.bcm = 0;
            self.line += 1;
            if self.line == 32 {
                self.line = 0;
            }

            // Load gamma-mapped pixel values into cache on line change.
            self.load_line();
        }

        // Render next BCM phase for mapped line to linebuffer.
        self.render_line();
    }

    /// Load gamma-mapped pixel values from framebuffer into the `gbuf` cache,
    /// for used by `render_line()`.
    fn load_line(&mut self) {
        let l1 = &self.fbuf.0[self.line as usize];
        let l2 = &self.fbuf.0[self.line as usize + 32];
        let cache = self.gbuf.chunks_exact_mut(6);
        for (([r1, g1, b1], [r2, g2, b2]), c) in l1.iter().zip(l2.iter()).zip(cache)
        {
            c[0] = GAMMA[*r1 as usize];
            c[1] = GAMMA[*g1 as usize];
            c[2] = GAMMA[*b1 as usize];
            c[3] = GAMMA[*r2 as usize];
            c[4] = GAMMA[*g2 as usize];
            c[5] = GAMMA[*b2 as usize];
        }
    }

    /// Render the current BCM phase of the gamma-mapped data cached in `gbuf`.
    fn render_line(&mut self) {
        let lbuf = &mut self.lbufs[self.lbuf as usize];
        let bcm = u8::min(self.bcm + self.bcm_skip, Self::BCM_PHASES);
        for (c, p) in self.gbuf.chunks_exact(6).zip(lbuf.iter_mut())
        {
            let r1 = ((c[0] >> bcm) & 1) as u8;
            let g1 = ((c[1] >> bcm) & 1) as u8;
            let b1 = ((c[2] >> bcm) & 1) as u8;
            let r2 = ((c[3] >> bcm) & 1) as u8;
            let g2 = ((c[4] >> bcm) & 1) as u8;
            let b2 = ((c[5] >> bcm) & 1) as u8;
            *p = r1 | (g1 << 1) | (b1 << 2) | (r2 << 3) | (g2 << 4) | (b2 << 5);
        }

        // Set latch on final cycle.
        lbuf[63] |= 1 << 6;

        // Clear all outputs on final byte.
        lbuf[64] = 0;
    }
}

/// Gamma lookup table, 8-bit input to 10-bit output.
///
/// To generate in Python:
///
/// ```python
/// import numpy as np
/// steps = 256
/// gamma = 3.0
/// max_in = (steps - 1)**gamma
/// max_out = 1023
/// tbl = (np.arange(steps)**gamma) * max_out/max_in
/// print(repr(tbl.round().astype(int)))
/// ```
pub static GAMMA: [u16; 256] = [
   0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,
   0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    1,
   1,    1,    1,    1,    1,    1,    1,    2,    2,    2,    2,
   2,    2,    3,    3,    3,    3,    4,    4,    4,    5,    5,
   5,    6,    6,    6,    7,    7,    8,    8,    9,    9,   10,
  10,   11,   11,   12,   13,   13,   14,   15,   15,   16,   17,
  18,   19,   19,   20,   21,   22,   23,   24,   25,   26,   27,
  28,   29,   30,   32,   33,   34,   35,   37,   38,   39,   41,
  42,   43,   45,   46,   48,   50,   51,   53,   55,   56,   58,
  60,   62,   64,   65,   67,   69,   71,   73,   76,   78,   80,
  82,   84,   87,   89,   91,   94,   96,   99,  101,  104,  107,
 109,  112,  115,  118,  120,  123,  126,  129,  132,  136,  139,
 142,  145,  148,  152,  155,  159,  162,  166,  169,  173,  177,
 180,  184,  188,  192,  196,  200,  204,  208,  212,  217,  221,
 225,  230,  234,  239,  243,  248,  253,  257,  262,  267,  272,
 277,  282,  287,  293,  298,  303,  308,  314,  319,  325,  331,
 336,  342,  348,  354,  360,  366,  372,  378,  384,  391,  397,
 403,  410,  417,  423,  430,  437,  444,  450,  457,  465,  472,
 479,  486,  494,  501,  509,  516,  524,  532,  539,  547,  555,
 563,  571,  580,  588,  596,  605,  613,  622,  630,  639,  648,
 657,  666,  675,  684,  693,  703,  712,  722,  731,  741,  751,
 760,  770,  780,  791,  801,  811,  821,  832,  842,  853,  864,
 874,  885,  896,  907,  918,  930,  941,  952,  964,  976,  987,
 999, 1011, 1023,
];
