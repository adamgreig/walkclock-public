use crate::{uart::Uart, gpio::OutputPin, dma::DMAStream};

/// Driver for uBlox MAX-M8 GNSS receiver.
pub struct UBlox {
    uart: Uart,
    reset: OutputPin,
    dma_stream: DMAStream,
    buf: &'static mut [u8; 100],
    pvt: Result<PVT, PVTError>,
    last_itow: u32,
}

impl UBlox {
    pub fn new(uart: Uart, reset: OutputPin, dma_stream: DMAStream, buf: &'static mut [u8; 100])
        -> Self
    {
        Self { uart, reset, dma_stream, buf, pvt: Err(PVTError::NoPVT(0)), last_itow: 0 }
    }

    /// Configure the uBlox.
    ///
    /// Disables NMEA messages, sets stationary dynamic mode,
    /// enables 50Hz timepulse and 1Hz PVT messages.
    pub fn setup(&self) {
        // Pulse RESET for ~1ms.
        self.reset.set_low();
        cortex_m::asm::delay(300_000);
        self.reset.set_high();

        // Clear any recently received byte, then wait for a new byte, indicating reset finished.
        self.uart.read_rdr();
        while !self.uart.rxne() {}

        // Configure UART port.
        static PRT: [u8; 26] = [
            // Sync 1, sync 2, class 6, ID 0x00, length 20
            0xb5, 0x62, 0x06, 0x00, 20, 0,
            // portID = Port 1 (USART1), reserved1, txReady=0
            0x01, 0x00, 0x00, 0x00,
            // mode = 8n1
            0b1100_0000, 0b0000_1000, 0x00, 0x00,
            // baudRate = 9600 = 0x0000_2580
            0x80, 0x25, 0x00, 0x00,
            // inProtoMask = UBX only
            0x01, 0x00,
            // outProtoMask = UBX only
            0x01, 0x00,
            // flags = 0
            0x00, 0x00,
            // reserved2
            0x00, 0x00,
        ];
        self.uart.write(&PRT);
        self.uart.write(&checksum(&PRT));

        // Wait for no further transmission before proceeding with configuration.
        self.uart.clear_idle();
        while !self.uart.idle() {}

        // Configure stationary dynamic model in NAV5.
        static NAV5: [u8; 42] = [
            // Sync 1, sync 2, class 6, ID 0x24, length 36
            0xb5, 0x62, 0x06, 0x24, 36, 0,
            // mask = 0x0001 = only apply dynamic model settings
            0x01, 0x00,
            // dynModel = stationary = 2
            2,
            // other settings masked
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];
        self.uart.write(&NAV5);
        self.uart.write(&checksum(&NAV5));

        // Configure 50Hz timepulse.
        static TP5: [u8; 38] = [
            // Sync 1, sync 2, class 6, ID 0x31, length 32
            0xb5, 0x62, 0x06, 0x31, 32, 0,
            // tpIdx = 0, version=1, reserved1
            0x00, 0x01, 0x00, 0x00,
            // antCableDelay=0, rfGroupDelay=0
            0x00, 0x00, 0x00, 0x00,
            // freqPeriod = 0x0000_0032 = 50Hz timepulse frequency
            0x32, 0x00, 0x00, 0x00,
            // freqPeriodLock = 0x0000_0032 = 50Hz timepulse frequency
            0x32, 0x00, 0x00, 0x00,
            // pulseLenRatio = 0x8000_0000 = 50% duty cycle
            0x00, 0x00, 0x00, 0x80,
            // pulseLenRatioLock = 0x8000_0000 = 50% duty cycle
            0x00, 0x00, 0x00, 0x80,
            // userConfigDelay = 0
            0x00, 0x00, 0x00, 0x00,
            // flags = 0b000_0100_1011
            //  bit 0: active = 1 = enable time pulse
            //  bit 1: lockGnssFreq = 1 = synchronise time pulse to GNSS
            //  bit 2: lockedOtherSet = 0 = use same values when locked as unlocked
            //  bit 3: isFreq = 1 = interpret freqPeriod as a frequency in Hz
            //  bit 4: isLength = 0 = interpret pulseLenRatio as duty cycle
            //  bit 5: alignToTow = 0 = don't align pulses to GPS ToW
            //  bit 6: polarity = 1 = rising edge at top of second
            //  bit 7-10: gridUtcGps = 0 = use UTC time grid
            //  bit 11: syncMode = 0 = ignored for non-FTS products
            0b0100_1011, 0b0000_0000, 0, 0,
        ];
        self.uart.write(&TP5);
        self.uart.write(&checksum(&TP5));

        // Configure 1Hz PVT messages.
        static MSG: [u8; 9] = [
            // Sync 1, sync 2, class 6, ID 0x01, length 3
            0xb5, 0x62, 0x06, 0x01, 3, 0,
            // msgClass = 1 = NAV, msgID = 7 = NAV-PVT, rate = 1
            0x01, 0x07, 1,
        ];
        self.uart.write(&MSG);
        self.uart.write(&checksum(&MSG));
    }

    /// Call to handle the RX DMA interrupt.
    ///
    /// This interrupt only fires on RX DMA completion, indicating
    /// a new UBX PVT frame is ready to parse.
    pub fn dma_isr(&mut self) {
        self.dma_stream.clear_tcif();

        // Parse PVT from received data.
        let pvt = PVT::try_from(self.buf);

        match pvt {
            // Save the new PVT unless its iTOW is the same as the last PVT, which
            // we reject to prevent processing duplicates as though they were new.
            Ok(pvt) => if pvt.itow != self.last_itow {
                self.pvt = Ok(pvt);
                self.last_itow = pvt.itow;
            } else {
                self.pvt = Err(PVTError::SameTOW)
            },

            // Save parse error otherwise.
            Err(e) => self.pvt = Err(e),
        }
    }

    /// Call to handle the UART interrupt.
    ///
    /// Only the IDLE interrupt is enabled, so this ISR is called when
    /// a line IDLE is detected, indicating we should start a new DMA
    /// reception transfer ready to receive the next data packet.
    pub fn uart_isr(&mut self) {
        self.uart.clear_idle();
        self.dma_stream.stop();
        self.uart.restart_dma_rx();
        self.dma_stream.start_rx(self.buf);
    }

    /// Take the most recently received PVT, if any.
    ///
    /// After calling, will return `PVTError::NoPVT` until a new PVT is received,
    /// with the u8 member incrementing each time.
    pub fn pvt(&mut self) -> Result<PVT, PVTError> {
        let pvt = self.pvt;
        self.pvt = match pvt {
            Err(PVTError::NoPVT(n)) => Err(PVTError::NoPVT(n.saturating_add(1))),
            _                       => Err(PVTError::NoPVT(0)),
        };
        pvt
    }
}

/// Parse the NAV-PVT packets sent by a uBlox chip.
#[derive(Copy, Clone, Debug)]
pub struct PVT {
    pub itow: u32,
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
    pub valid_date: bool,
    pub valid_time: bool,
    pub fully_resolved: bool,
    pub fix: bool,
    pub num_sv: u8,
}

#[derive(Copy, Clone, Debug)]
pub enum PVTError {
    /// Sync bytes in most recent frame were wrong.
    BadSync,
    /// Class or ID bytes in most recent frame were wrong.
    BadClassID,
    /// Length bytes in most recent frame were wrong.
    BadLength,
    /// Checksum in most recent frame was wrong.
    BadChecksum,
    /// iTOW on most recent frame was not different from last PVT.
    SameTOW,
    /// No PVT frame has been received.
    /// The u8 counts how many times NoPVT is returned.
    NoPVT(u8),
}

impl PVT {
    fn try_from(buf: &[u8; 100]) -> Result<Self, PVTError> {
        if buf[0] != 0xB5 || buf[1] != 0x62 {
            return Err(PVTError::BadSync);
        }
        if buf[2] != 0x01 || buf[3] != 0x07 {
            return Err(PVTError::BadClassID);
        }
        if buf[4] != 92 || buf[5] != 0 {
            return Err(PVTError::BadLength);
        }
        if [buf[98], buf[99]] != checksum(&buf[..98]) {
            return Err(PVTError::BadChecksum);
        }

        let buf = &buf[6..98];

        Ok(PVT {
            itow: u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]),
            year: u16::from_le_bytes([buf[4], buf[5]]),
            month: buf[6],
            day: buf[7],
            hour: buf[8],
            minute: buf[9],
            second: buf[10],
            valid_date: buf[11] & 0b0001 != 0,
            valid_time: buf[11] & 0b0010 != 0,
            fully_resolved: buf[11] & 0b0100 != 0,
            fix: buf[20] > 1,
            num_sv: buf[23],
        })
    }
}

/// Compute the checksum for a message.
///
/// The message should include the normal leading sync bytes
/// but not include any space at the end for a checksum.
fn checksum(msg: &[u8]) -> [u8; 2] {
    let (mut a, mut b) = (0u8, 0u8);
    for byte in &msg[2..] {
        a = a.wrapping_add(*byte);
        b = b.wrapping_add(a);
    }
    [a, b]
}
