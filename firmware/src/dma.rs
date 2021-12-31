use stm32ral::{dma, dmamux1, write_reg, read_reg, modify_reg};

/// Driver for the DMAMUX1 peripheral.
pub struct DMAMux1 {
    dmamux1: dmamux1::Instance,
}

impl DMAMux1 {
    /// Create a new DMAMux1 driver.
    pub fn new(dmamux1: dmamux1::Instance) -> Self {
        Self { dmamux1 }
    }

    /// Configures the requested channel, which must be in 0..15, to mux the
    /// requested DMAREQ ID.
    ///
    /// Does not enable or support synchronisation or event generation.
    pub fn set(&self, channel: u32, id: u32) {
        match channel {
            0 => write_reg!(dmamux1, self.dmamux1, CCR0, DMAREQ_ID: id),
            1 => write_reg!(dmamux1, self.dmamux1, CCR1, DMAREQ_ID: id),
            2 => write_reg!(dmamux1, self.dmamux1, CCR2, DMAREQ_ID: id),
            3 => write_reg!(dmamux1, self.dmamux1, CCR3, DMAREQ_ID: id),
            4 => write_reg!(dmamux1, self.dmamux1, CCR4, DMAREQ_ID: id),
            5 => write_reg!(dmamux1, self.dmamux1, CCR5, DMAREQ_ID: id),
            6 => write_reg!(dmamux1, self.dmamux1, CCR6, DMAREQ_ID: id),
            7 => write_reg!(dmamux1, self.dmamux1, CCR7, DMAREQ_ID: id),
            8 => write_reg!(dmamux1, self.dmamux1, CCR8, DMAREQ_ID: id),
            9 => write_reg!(dmamux1, self.dmamux1, CCR9, DMAREQ_ID: id),
            10 => write_reg!(dmamux1, self.dmamux1, CCR10, DMAREQ_ID: id),
            11 => write_reg!(dmamux1, self.dmamux1, CCR11, DMAREQ_ID: id),
            12 => write_reg!(dmamux1, self.dmamux1, CCR12, DMAREQ_ID: id),
            13 => write_reg!(dmamux1, self.dmamux1, CCR13, DMAREQ_ID: id),
            14 => write_reg!(dmamux1, self.dmamux1, CCR14, DMAREQ_ID: id),
            15 => write_reg!(dmamux1, self.dmamux1, CCR15, DMAREQ_ID: id),
            _ => panic!("Unknown DMAMUX1 channel {}", channel),
        }
    }
}

/// Safe construction of all 8 streams in a DMA peripheral.
pub struct DMA {
    pub s0: DMAStream,
    pub s1: DMAStream,
    pub s2: DMAStream,
    pub s3: DMAStream,
    pub s4: DMAStream,
    pub s5: DMAStream,
    pub s6: DMAStream,
    pub s7: DMAStream,
}

impl DMA {
    /// Create the set of streams for a DMA peripheral, consuming it in the process.
    pub fn new(dma: dma::Instance) -> Self {
        // NOTE(unsafe): We just have to ensure only one DMAStream instance
        // NOTE(unsafe): is created for each DMA stream.
        unsafe {
            Self {
                s0: DMAStream::new(&dma, 0),
                s1: DMAStream::new(&dma, 1),
                s2: DMAStream::new(&dma, 2),
                s3: DMAStream::new(&dma, 3),
                s4: DMAStream::new(&dma, 4),
                s5: DMAStream::new(&dma, 5),
                s6: DMAStream::new(&dma, 6),
                s7: DMAStream::new(&dma, 7),
            }
        }
    }
}

/// Driver for controlling a DMA stream.
pub struct DMAStream {
    dma: dma::Instance,
    stream: usize,
}

impl DMAStream {
    /// Create a new DMAStream for the provided dma instance and stream number.
    ///
    /// # Safety
    /// Must only create one instance per stream.
    pub unsafe fn new(dma: &dma::Instance, stream: usize) -> DMAStream {
        // NOTE(unsafe): Make a copy of `dma` which we will only modify
        // NOTE(unsafe): in ways relating exclusively to our stream.
        let dma = core::mem::transmute_copy(dma);
        DMAStream { dma, stream }
    }

    /// Set up this stream for transmit (memory-to-peripheral) operation.
    /// Configures 8-bit reads and writes, increments memory, and uses the FIFO.
    pub fn setup_tx(&self, par0: u32) {
        let stream = self.stream();
        write_reg!(dma, stream, CR0, EN: Disabled);
        while read_reg!(dma, stream, CR0, EN != Disabled) {}
        write_reg!(dma, stream, CR0,
            MBURST: INCR8, MSIZE: Bits8, PSIZE: Bits8, MINC: Incremented, PINC: Fixed,
            DIR: MemoryToPeripheral, TCIE: Enabled, EN: Disabled);
        write_reg!(dma, stream, PAR0, par0);
        write_reg!(dma, stream, FCR0, FEIE: Disabled, DMDIS: 1, FTH: Half);
    }

    /// Set up this stream for receive (peripheral-to-memory) operation.
    /// Configures 8-bit reads and writes, increments memory, and uses the FIFO.
    pub fn setup_rx(&self, par0: u32) {
        let stream = self.stream();
        write_reg!(dma, stream, CR0, EN: Disabled);
        while read_reg!(dma, stream, CR0, EN != Disabled) {}
        write_reg!(dma, stream, CR0,
            MBURST: INCR8, MSIZE: Bits8, PSIZE: Bits8, MINC: Incremented, PINC: Fixed,
            DIR: PeripheralToMemory, TCIE: Enabled, EN: Disabled);
        write_reg!(dma, stream, PAR0, par0);
        write_reg!(dma, stream, FCR0, FEIE: Disabled, DMDIS: 1, FTH: Half);
    }

    /// Set up this stream for receive (peripheral-to-memory) operation.
    /// Configures 16-bit reads and writes, increments memory, and uses the FIFO.
    pub fn setup_u16_rx(&self, par0: u32) {
        let stream = self.stream();
        write_reg!(dma, stream, CR0, EN: Disabled);
        while read_reg!(dma, stream, CR0, EN != Disabled) {}
        write_reg!(dma, stream, CR0,
            MBURST: INCR4, MSIZE: Bits16, PSIZE: Bits16, MINC: Incremented, PINC: Fixed,
            DIR: PeripheralToMemory, TCIE: Enabled, EN: Disabled);
        write_reg!(dma, stream, PAR0, par0);
        write_reg!(dma, stream, FCR0, FEIE: Disabled, DMDIS: 1, FTH: Half);
    }

    /// Enable the TRBUFF bit required for UART streams.
    pub fn set_trbuff(&self) {
        let stream = self.stream();
        modify_reg!(dma, stream, CR0, TRBUFF: Enabled);
    }

    /// Start this stream for transmit (memory-to-peripheral) operation,
    /// using the provided slice's address and length.
    pub fn start_tx(&self, m0ar0: &[u8]) {
        self.clear_flags();
        let stream = self.stream();
        write_reg!(dma, stream, M0AR0, m0ar0.as_ptr() as u32);
        write_reg!(dma, stream, NDTR0, m0ar0.len() as u32);
        modify_reg!(dma, stream, CR0, EN: Enabled);
    }

    /// Start this stream for receive (peripheral-to-memory) operation,
    /// using the provided slice's address and length.
    pub fn start_rx(&self, m0ar0: &mut [u8]) {
        self.clear_flags();
        let stream = self.stream();
        write_reg!(dma, stream, M0AR0, m0ar0.as_ptr() as u32);
        write_reg!(dma, stream, NDTR0, m0ar0.len() as u32);
        modify_reg!(dma, stream, CR0, EN: Enabled);
    }

    /// Start this stream for receive (peripheral-to-memory) operation,
    /// with 16-bit data, using the provided slice's address and length.
    pub fn start_u16_rx(&self, m0ar0: &mut [u16]) {
        self.clear_flags();
        let stream = self.stream();
        write_reg!(dma, stream, M0AR0, m0ar0.as_ptr() as u32);
        write_reg!(dma, stream, NDTR0, m0ar0.len() as u32);
        modify_reg!(dma, stream, CR0, EN: Enabled);
    }

    /// Cancel any ongoing DMA transfer.
    pub fn stop(&self) {
        let stream = self.stream();
        modify_reg!(dma, stream, CR0, EN: Disabled);
        while read_reg!(dma, stream, CR0, EN != Disabled) {}
    }

    /// Get the value of the TCIF flag for this stream.
    pub fn tcif(&self) -> bool {
        match self.stream {
            0 => read_reg!(dma, self.dma, LISR, TCIF0 == Complete),
            1 => read_reg!(dma, self.dma, LISR, TCIF1 == Complete),
            2 => read_reg!(dma, self.dma, LISR, TCIF2 == Complete),
            3 => read_reg!(dma, self.dma, LISR, TCIF3 == Complete),
            4 => read_reg!(dma, self.dma, HISR, TCIF4 == Complete),
            5 => read_reg!(dma, self.dma, HISR, TCIF5 == Complete),
            6 => read_reg!(dma, self.dma, HISR, TCIF6 == Complete),
            7 => read_reg!(dma, self.dma, HISR, TCIF7 == Complete),
            _ => false,
        }
    }

    /// Get the value of the TCIF flag for this stream.
    pub fn flags(&self) -> u32 {
        match self.stream {
            0..=3 => read_reg!(dma, self.dma, LISR),
            4..=7 => read_reg!(dma, self.dma, HISR),
            _ => 0,
        }
    }

    /// Clear transfer-complete flag for this stream.
    pub fn clear_tcif(&self) {
        match self.stream {
            0 => write_reg!(dma, self.dma, LIFCR, CTCIF0: Clear),
            1 => write_reg!(dma, self.dma, LIFCR, CTCIF1: Clear),
            2 => write_reg!(dma, self.dma, LIFCR, CTCIF2: Clear),
            3 => write_reg!(dma, self.dma, LIFCR, CTCIF3: Clear),
            4 => write_reg!(dma, self.dma, HIFCR, CTCIF4: Clear),
            5 => write_reg!(dma, self.dma, HIFCR, CTCIF5: Clear),
            6 => write_reg!(dma, self.dma, HIFCR, CTCIF6: Clear),
            7 => write_reg!(dma, self.dma, HIFCR, CTCIF7: Clear),
            _ => unreachable!(),
        }
    }

    /// Clear all flags for this stream.
    pub fn clear_flags(&self) {
        match self.stream {
            0 => write_reg!(dma, self.dma, LIFCR, 0x0000_003D),
            1 => write_reg!(dma, self.dma, LIFCR, 0x0000_0F40),
            2 => write_reg!(dma, self.dma, LIFCR, 0x003D_0000),
            3 => write_reg!(dma, self.dma, LIFCR, 0x0F40_0000),
            4 => write_reg!(dma, self.dma, HIFCR, 0x0000_003D),
            5 => write_reg!(dma, self.dma, HIFCR, 0x0000_0F40),
            6 => write_reg!(dma, self.dma, HIFCR, 0x003D_0000),
            7 => write_reg!(dma, self.dma, HIFCR, 0x0F40_0000),
            _ => unreachable!(),
        }
    }

    /// Return a special dma::Instance where the 0th stream register
    /// maps to our specific stream.
    ///
    /// Do not access LISR/HISR/LIFCR/HIFCR through this instance!
    fn stream(&self) -> dma::Instance {
        let ptr = &*self.dma as *const _ as *const u32;
        unsafe { core::mem::transmute(ptr.offset(6 * self.stream as isize)) }
    }
}
