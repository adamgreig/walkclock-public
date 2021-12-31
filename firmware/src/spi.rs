use stm32ral::{spi, write_reg, read_reg, modify_reg};
use crate::rcc::Clocks;

/// SPI driver.
pub struct Spi {
    spi: spi::Instance,
}

impl Spi {
    pub fn new(spi: spi::Instance) -> Self {
        Self { spi }
    }

    pub fn setup_lcd(&self, clocks: &Clocks) {
        // Ensure SPI is disabled and all CR1 settings at default.
        write_reg!(spi, self.spi, CR1, SPE: Disabled);

        // Use arbitrary transfer sizes.
        write_reg!(spi, self.spi, CR2, TSER: 0, TSIZE: 0);

        // Use 75M/8=9.375MHz clock, LCD maximum is 15MHz. Note DSIZE=7 for 8-bit data.
        assert!(clocks.spi4_ck / 8 < 15_000_000);
        write_reg!(spi, self.spi, CFG1,
            MBR: Div8, TXDMAEN: Enabled, DSIZE: 8 - 1);
        write_reg!(spi, self.spi, CFG2,
            SSOM: Asserted, SSOE: Enabled, SSM: Disabled, CPOL: IdleLow, CPHA: FirstEdge,
            LSBFRST: MSBFirst, MASTER: Master, COMM: Transmitter);

        // Enable SPI.
        write_reg!(spi, self.spi, CR1, SPE: Enabled);
    }

    /// Transmit one slice of bytes out SPI, blocking until transmission completes.
    ///
    /// The slice is transmitted byte-by-byte without DMA and so there are no restrictions
    /// on the data storage location.
    pub fn write(&self, data: &[u8]) {
        // Start transaction.
        self.start_tx();

        for word in data.iter() {
            // NOTE(unsafe): Write to a register we already have exclusive access to via
            // NOTE(unsafe): ownership of self.spi, but we need to ensure a u8-sized write.
            unsafe { core::ptr::write_volatile(&self.spi.TXDR as *const _ as *mut u8, *word) };

            // Wait for TxFIFO to have room for the next write.
            while read_reg!(spi, self.spi, SR, TXP == Full) {}
        }

        // End transaction.
        self.end_tx();
    }

    /// Start a transmission transaction.
    ///
    /// Useful before starting a DMA operation. Asserts hardware CS
    /// and enables transmission of data written to TxFIFO.
    pub fn start_tx(&self) {
        modify_reg!(spi, self.spi, CR1, SPE: Enabled, CSTART: Started);
    }

    /// Finish a transmission transaction.
    ///
    /// Useful after a DMA operation. Deasserts hardware CS.
    pub fn end_tx(&self) {
        // Wait for transmission out of TxFIFO to finish.
        while read_reg!(spi, self.spi, SR, TXC == Ongoing) {}

        // Request and wait for end of transaction.
        modify_reg!(spi, self.spi, CR1, CSUSP: Requested);
        while read_reg!(spi, self.spi, SR, SUSP == NotSuspended) {}
        write_reg!(spi, self.spi, IFCR, SUSPC: Clear);
    }

    /// Check if a transmission is complete.
    ///
    /// Returns true when transmission is complete, i.e. not currently busy.
    pub fn txc(&self) -> bool {
        read_reg!(spi, self.spi, SR, TXC == Completed)
    }

    /// Get the address of this SPI's TXDR register.
    pub fn txdr(&self) -> u32 {
        &self.spi.TXDR as *const _ as u32
    }
}
