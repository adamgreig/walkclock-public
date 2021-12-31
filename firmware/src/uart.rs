use stm32ral::{usart, read_reg, write_reg, modify_reg};
use crate::rcc::Clocks;

/// UART driver.
pub struct Uart {
    uart: usart::Instance,
}

impl Uart {
    pub fn new(uart: usart::Instance) -> Self {
        Self { uart }
    }

    pub fn setup_ublox(&self, clocks: &Clocks) {
        // Configure UART for FIFO-disabled, 16x oversampling, 8-bit data, IDLE interrupt.
        write_reg!(usart, self.uart, CR1, FIFOEN: 0, OVER8: Oversampling16, M1: M0, M0: Bit8,
                                          IDLEIE: Enabled, UE: Disabled);

        // Use all default settings for CR2.
        write_reg!(usart, self.uart, CR2, 0);

        // Disable overrun errors, since we can't do anything about them anyway
        // and will recovery naturally as more data comes in.
        // Enable DMA for received data.
        write_reg!(usart, self.uart, CR3, OVRDIS: Disabled, DMAR: Enabled);

        // Configure for 9600 baud.
        write_reg!(usart, self.uart, BRR, clocks.uart8_ck / 9600);

        // Enable UART.
        modify_reg!(usart, self.uart, CR1, TE: Enabled, RE: Enabled, UE: Enabled);
    }

    /// Restart DMA reception, clearing any pending data.
    pub fn restart_dma_rx(&self) {
        modify_reg!(usart, self.uart, CR3, DMAR: Disabled);
        write_reg!(usart, self.uart, RQR, RXFRQ: Discard);
        modify_reg!(usart, self.uart, CR3, DMAR: Enabled);
    }

    /// Get address of receive data register.
    pub fn rdr(&self) -> u32 {
        &self.uart.RDR as *const _ as u32
    }

    /// Get address of transmit data register.
    /// Read byte from receive data register.
    pub fn read_rdr(&self) -> u8 {
        read_reg!(usart, self.uart, RDR) as u8
    }

    /// Write byte into the transmit data register.
    pub fn write_tdr(&self, byte: u8) {
        write_reg!(usart, self.uart, TDR, byte as u32);
    }

    /// Read state of RXNE flag.
    ///
    /// This flag indicates new data is present in RDR.
    pub fn rxne(&self) -> bool {
        read_reg!(usart, self.uart, ISR, RXNE == 1)
    }

    /// Read state of TXE flag.
    ///
    /// This flag indicates the data written to TDR has been transferred to
    /// the transmission shift register, so new data may be written to TDR
    /// but the original data has not necessarily finished transmitting.
    pub fn txe(&self) -> bool {
        read_reg!(usart, self.uart, ISR, TXE == 1)
    }

    /// Read state of IDLE flag.
    ///
    /// This flag is set when an idle line is detected.
    /// Clear by calling `clear_idle()`.
    pub fn idle(&self) -> bool {
        read_reg!(usart, self.uart, ISR, IDLE == 1)
    }

    /// Clear the IDLE flag.
    ///
    /// The flag will not be re-set until after further data is received.
    pub fn clear_idle(&self) {
        write_reg!(usart, self.uart, ICR, IDLECF: Clear);
    }

    /// Blocking write of slice of data.
    ///
    /// Returns once the final byte is written to the TDR register,
    /// but this may be before transmission completes to the wire.
    pub fn write(&self, data: &[u8]) {
        for byte in data.iter() {
            while !self.txe() {}
            self.write_tdr(*byte);
        }
    }
}
