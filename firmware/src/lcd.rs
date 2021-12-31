use crate::{spi::Spi, gpio::OutputPin, dma::DMAStream, framebuf::SubFrameBuf};

/// Driver for ST7735S LCD controller attached via 4-wire SPI.
pub struct Lcd {
    spi: Spi,
    dcx: OutputPin,
    dma_stream: DMAStream,
}

impl Lcd {
    pub fn new(spi: Spi, dcx: OutputPin, dma_stream: DMAStream)
        -> Self
    {
        Self { spi, dcx, dma_stream }
    }

    /// Call to set up and then begin rendering the provided framebuffer to the LCD.
    pub fn start(&self) {
        // Initialise LCD configuration.
        self.setup();
    }

    /// Call to handle the DMA transfer completion ISR.
    pub fn dma_isr(&mut self) {
        self.dma_stream.clear_tcif();

        // End previous DMA transaction.
        self.spi.end_tx();
    }

    /// Write provided framebuffer to the LCD.
    ///
    /// This method blocks briefly to transmit the memory-write command over
    /// SPI, then sets up a DMA transfer for the framebuffer data itself.
    ///
    /// If a previous transmission is still ongoing, no action is taken.
    pub fn write_fbuf(&self, fbuf: &'static SubFrameBuf) {
        if self.spi.txc() {
            self.command(Command::WriteRam, &[]);
            self.spi.start_tx();
            self.dma_stream.start_tx(fbuf.as_slice());
        }
    }

    /// Configure LCD.
    fn setup(&self) {
        // Trigger a reset.
        self.command(Command::SwReset, &[]);

        // Wait approx 120ms for display to finish resetting.
        cortex_m::asm::delay(40_000_000);

        // Leave sleep mode.
        self.command(Command::SleepOut, &[0x00]);

        // Set frame rate to normal mode. Just some magic numbers.
        self.command(Command::FrameRateCtrl1, &[0x01, 0x2c, 0x2d]);
        self.command(Command::FrameRateCtrl2, &[0x01, 0x2c, 0x2d]);
        self.command(Command::FrameRateCtrl3, &[0x01, 0x2c, 0x2d, 0x01, 0x2d, 0x2d]);

        // This display panel requires inversion.
        self.command(Command::FrameInversionCtrl, &[0x07]);
        self.command(Command::DisplayInversionOn, &[]);

        // Power control to -4.6V AUTO. More magic numbers.
        self.command(Command::PwrCtrl1, &[0xa2, 0x02, 0x84]);
        self.command(Command::PwrCtrl2, &[0xc5]);
        self.command(Command::PwrCtrl3, &[0x0a, 0x00]);
        self.command(Command::PwrCtrl4, &[0x8a, 0x2a]);
        self.command(Command::PwrCtrl5, &[0x8a, 0xee]);
        self.command(Command::VcomhVcomlCtrl1, &[0x0e]);

        // Colour mode to 18 bits/pixel, which then expects an RGB888 data stream.
        self.command(Command::ColorMode, &[0b110]);

        // Gamma map. Magic numbers.
        self.command(Command::PvGammaCtrl, &[
            0x02, 0x1c, 0x07, 0x12, 0x37, 0x32, 0x29, 0x2d,
            0x29, 0x25, 0x2b, 0x39, 0x00, 0x01, 0x03, 0x10,
        ]);
        self.command(Command::NvGammaCtrl, &[
            0x03, 0x1d, 0x07, 0x06, 0x2e, 0x2c, 0x29, 0x2d,
            0x2e, 0x2e, 0x37, 0x3f, 0x00, 0x00, 0x02, 0x10,
        ]);

        // Set display window. 80 rows by 160 columns, plus a mystery 26 and 1 offset.
        self.command(Command::RaSet, &[0, 26, 0, 26 + 80 - 1]);
        self.command(Command::CaSet, &[0, 1, 0, 1 + 160 - 1]);

        // Set memory data access control: scan order, colour order.
        // We set MY to flip rows, MV to row/col exchange, RGB to set BGR color filter.
        self.command(Command::MadCtl, &[0b1010_1000]);

        // Display on.
        self.command(Command::NormalDisplayOff, &[0x00]);
        self.command(Command::DisplayOn, &[0x00]);
    }

    /// Issue LCD command, with optional data.
    fn command(&self, command: Command, data: &[u8]) {
        self.dcx.set_low();
        self.spi.write(&[command as u8]);
        self.dcx.set_high();
        if !data.is_empty() {
            self.spi.write(data);
        }
    }
}

/// List of available ST7735S commands.
#[repr(u8)]
#[derive(Copy, Clone)]
#[allow(unused)]
enum Command {
    Nop = 0x00,
    SwReset = 0x01,
    SleepIn = 0x10,
    SleepOut = 0x11,
    NormalDisplayOff = 0x13,
    DisplayInversionOff = 0x20,
    DisplayInversionOn = 0x21,
    GammaSet = 0x26,
    DisplayOff = 0x28,
    DisplayOn = 0x29,
    CaSet = 0x2A,
    RaSet = 0x2B,
    WriteRam = 0x2C,
    RgbSet = 0x2D,
    MadCtl = 0x36,
    IdleModeOff = 0x38,
    IdleModeOn = 0x39,
    ColorMode = 0x3A,
    FrameRateCtrl1 = 0xB1,
    FrameRateCtrl2 = 0xB2,
    FrameRateCtrl3 = 0xB3,
    FrameInversionCtrl = 0xB4,
    DisplaySetting = 0xB6,
    PwrCtrl1 = 0xC0,
    PwrCtrl2 = 0xC1,
    PwrCtrl3 = 0xC2,
    PwrCtrl4 = 0xC3,
    PwrCtrl5 = 0xC4,
    VcomhVcomlCtrl1 = 0xC5,
    VmofCtrl = 0xC7,
    PvGammaCtrl = 0xE0,
    NvGammaCtrl = 0xE1,
    PwrCtrl6 = 0xFC,
    Vcom4Level = 0xFF,
}
