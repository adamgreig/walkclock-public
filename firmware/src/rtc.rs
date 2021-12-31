use stm32ral::{rtc, read_reg, write_reg, modify_reg};
use crate::ublox::PVT;

/// Date and time read from RTC.
///
/// Note that year is in years since 2000.
#[derive(Copy, Clone, Debug)]
pub struct DateTime {
    pub year: u8,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
}

impl From<&PVT> for DateTime {
    fn from(pvt: &PVT) -> Self {
        Self {
            year: (pvt.year - 2000) as u8,
            month: pvt.month,
            day: pvt.day,
            hour: pvt.hour,
            minute: pvt.minute,
            second: pvt.second,
        }
    }
}

impl DateTime {
    /// Compare two datetimes, returning true if they differ by more than two seconds.
    fn different(a: &DateTime, b: &DateTime) -> bool {
        // To avoid tricky comparisons around rollovers, always return false
        // when within 2 seconds of a new minute.
        if a.near_new_minute() || b.near_new_minute() {
            return false;
        }

        // If not near a new minute, we can only be close if all other fields
        // match perfectly and seconds are no more than 2 seconds apart.
        let secs = i8::abs(a.second as i8 - b.second as i8);
        (a.year != b.year) || (a.month != b.month) || (a.day != b.day) ||
        (a.hour != b.hour) || (a.minute != b.minute) || secs > 2
    }

    /// Check if this DateTime is within two seconds of changing minute.
    fn near_new_minute(&self) -> bool {
        self.second > 57 || self.second < 2
    }
}

/// Driver for the STM32H7 RTC peripheral.
#[allow(clippy::upper_case_acronyms)]
pub struct RTC {
    rtc: rtc::Instance,
}

impl RTC {
    /// Set up the RTC peripheral.
    pub fn new(rtc: rtc::Instance) -> Self {
        // Unlock RTC registers.
        write_reg!(rtc, rtc, WPR, KEY: 0xCA);
        write_reg!(rtc, rtc, WPR, KEY: 0x53);

        // Enter initialisation mode, where the time, date, prescaler,
        // and reference enable may be programmed.
        write_reg!(rtc, rtc, ISR, INIT: InitMode);
        while read_reg!(rtc, rtc, ISR, INITF != Allowed) {}

        // Enable reference clock detection.
        write_reg!(rtc, rtc, CR, REFCKON: Enabled);

        // Set prescalers to default values of A=0x7F S=0xFF,
        // giving ck_apre=32768/128=256Hz, ck_spre=ck_apre/256=1Hz.
        write_reg!(rtc, rtc, PRER, PREDIV_A: 0x7F, PREDIV_S: 0xFF);

        // Output 1Hz calibration signal if enabled.
        if cfg!(feature = "rtc_coe") {
            modify_reg!(rtc, rtc, CR, COE: 1, COSEL: CalFreq_1Hz);
        }

        // Leave initialisation mode and begin running the clock.
        write_reg!(rtc, rtc, ISR, INIT: FreeRunningMode);

        // Re-lock RTC.
        write_reg!(rtc, rtc, WPR, KEY: 0);

        Self { rtc }
    }

    /// Read the current RTC date and time.
    pub fn read(&self) -> DateTime {
        // Wait for valid values to be loaded into the shadow registers.
        while read_reg!(rtc, self.rtc, ISR, RSF != Synced) {}

        // Read new value for time and date.
        // Note reading TR locks DR until it is read.
        let tr = read_reg!(rtc, self.rtc, TR);
        let dr = read_reg!(rtc, self.rtc, DR);

        // Clear RSF to ensure fresh values next read.
        modify_reg!(rtc, self.rtc, ISR, RSF: Clear);

        let yt = (dr >> 20) & 0b1111;
        let yu = (dr >> 16) & 0b1111;
        let mt = (dr >> 12) & 0b1;
        let mu = (dr >> 8)  & 0b1111;
        let dt = (dr >> 4)  & 0b11;
        let du = dr         & 0b1111;

        let ht = (tr >> 20) & 0b11;
        let hu = (tr >> 16) & 0b1111;
        let mnt = (tr >> 12) & 0b111;
        let mnu = (tr >> 8)  & 0b1111;
        let st = (tr >> 4)  & 0b111;
        let su = tr         & 0b1111;

        DateTime {
            year: ((yt * 10) + yu) as u8,
            month: ((mt * 10) + mu) as u8,
            day: ((dt * 10) + du) as u8,
            hour: ((ht * 10) + hu) as u8,
            minute: ((mnt * 10) + mnu) as u8,
            second: ((st * 10) + su) as u8,
        }
    }

    /// Set the RTC to the provided date and time.
    pub fn set(&self, date: &DateTime) {
        // Compute BCD format for date and time.
        let yt = (date.year / 10) as u32;
        let yu = (date.year % 10) as u32;
        let mt = (date.month / 10) as u32;
        let mu = (date.month % 10) as u32;
        let dt = (date.day / 10) as u32;
        let du = (date.day % 10) as u32;
        let ht = (date.hour / 10) as u32;
        let hu = (date.hour % 10) as u32;
        let mnt = (date.minute / 10) as u32;
        let mnu = (date.minute % 10) as u32;
        let st = (date.second / 10) as u32;
        let su = (date.second % 10) as u32;

        // Unlock RTC registers.
        write_reg!(rtc, self.rtc, WPR, KEY: 0xCA);
        write_reg!(rtc, self.rtc, WPR, KEY: 0x53);

        // Enter initialisation mode, where the time, date, prescaler,
        // and reference enable may be programmed.
        write_reg!(rtc, self.rtc, ISR, INIT: InitMode);
        while read_reg!(rtc, self.rtc, ISR, INITF != Allowed) {}

        // Write date and time registers.
        write_reg!(rtc, self.rtc, TR, HT: ht, HU: hu, MNT: mnt, MNU: mnu, ST: st, SU: su);
        write_reg!(rtc, self.rtc, DR, YT: yt, YU: yu, MT: mt, MU: mu, DT: dt, DU: du);

        // Leave initialisation mode and begin running the clock.
        write_reg!(rtc, self.rtc, ISR, INIT: FreeRunningMode);

        // Re-lock RTC.
        write_reg!(rtc, self.rtc, WPR, KEY: 0);
    }

    /// Process receiving a new PVT frame from the GNSS.
    ///
    /// If the PVT is fully valid and in lock, and the PVT time is more than two
    /// seconds different to the RTC time, the RTC is re-set to the PVT time.
    pub fn new_pvt(&self, pvt: &PVT) {
        // Ignore not-fully-valid PVTs.
        if !pvt.fix || !pvt.valid_date || !pvt.valid_time || !pvt.fully_resolved {
            return;
        }

        let rtc = self.read();
        let pvt = DateTime::from(pvt);
        if DateTime::different(&rtc, &pvt) {
            rtt_target::rprintln!("RTC {} and PVT {} differ, resetting RTC", rtc, pvt);
            self.set(&pvt);
        }
    }

    /// Read backup registers into `data`.
    ///
    /// Up to 32 backup registers are read, depending on the length of `data`.
    pub fn read_backup(&self, data: &mut [u32]) {
        for (idx, d) in data.iter_mut().take(32).enumerate() {
            match idx {
                0 => *d = read_reg!(rtc, self.rtc, BKP0R),
                1 => *d = read_reg!(rtc, self.rtc, BKP1R),
                2 => *d = read_reg!(rtc, self.rtc, BKP2R),
                3 => *d = read_reg!(rtc, self.rtc, BKP3R),
                4 => *d = read_reg!(rtc, self.rtc, BKP4R),
                5 => *d = read_reg!(rtc, self.rtc, BKP5R),
                6 => *d = read_reg!(rtc, self.rtc, BKP6R),
                7 => *d = read_reg!(rtc, self.rtc, BKP7R),
                8 => *d = read_reg!(rtc, self.rtc, BKP8R),
                9 => *d = read_reg!(rtc, self.rtc, BKP9R),
                10 => *d = read_reg!(rtc, self.rtc, BKP10R),
                11 => *d = read_reg!(rtc, self.rtc, BKP11R),
                12 => *d = read_reg!(rtc, self.rtc, BKP12R),
                13 => *d = read_reg!(rtc, self.rtc, BKP13R),
                14 => *d = read_reg!(rtc, self.rtc, BKP14R),
                15 => *d = read_reg!(rtc, self.rtc, BKP15R),
                16 => *d = read_reg!(rtc, self.rtc, BKP16R),
                17 => *d = read_reg!(rtc, self.rtc, BKP17R),
                18 => *d = read_reg!(rtc, self.rtc, BKP18R),
                19 => *d = read_reg!(rtc, self.rtc, BKP19R),
                20 => *d = read_reg!(rtc, self.rtc, BKP20R),
                21 => *d = read_reg!(rtc, self.rtc, BKP21R),
                22 => *d = read_reg!(rtc, self.rtc, BKP22R),
                23 => *d = read_reg!(rtc, self.rtc, BKP23R),
                24 => *d = read_reg!(rtc, self.rtc, BKP24R),
                25 => *d = read_reg!(rtc, self.rtc, BKP25R),
                26 => *d = read_reg!(rtc, self.rtc, BKP26R),
                27 => *d = read_reg!(rtc, self.rtc, BKP27R),
                28 => *d = read_reg!(rtc, self.rtc, BKP28R),
                29 => *d = read_reg!(rtc, self.rtc, BKP29R),
                30 => *d = read_reg!(rtc, self.rtc, BKP30R),
                31 => *d = read_reg!(rtc, self.rtc, BKP31R),
                _ => (),
            }
        }
    }

    /// Write backup registers from `data`.
    ///
    /// Up to 32 backup registers are written, depending on the length of `data`.
    pub fn write_backup(&self, data: &[u32]) {
        for (idx, d) in data.iter().take(32).enumerate() {
            match idx {
                0 => write_reg!(rtc, self.rtc, BKP0R, *d),
                1 => write_reg!(rtc, self.rtc, BKP1R, *d),
                2 => write_reg!(rtc, self.rtc, BKP2R, *d),
                3 => write_reg!(rtc, self.rtc, BKP3R, *d),
                4 => write_reg!(rtc, self.rtc, BKP4R, *d),
                5 => write_reg!(rtc, self.rtc, BKP5R, *d),
                6 => write_reg!(rtc, self.rtc, BKP6R, *d),
                7 => write_reg!(rtc, self.rtc, BKP7R, *d),
                8 => write_reg!(rtc, self.rtc, BKP8R, *d),
                9 => write_reg!(rtc, self.rtc, BKP9R, *d),
                10 => write_reg!(rtc, self.rtc, BKP10R, *d),
                11 => write_reg!(rtc, self.rtc, BKP11R, *d),
                12 => write_reg!(rtc, self.rtc, BKP12R, *d),
                13 => write_reg!(rtc, self.rtc, BKP13R, *d),
                14 => write_reg!(rtc, self.rtc, BKP14R, *d),
                15 => write_reg!(rtc, self.rtc, BKP15R, *d),
                16 => write_reg!(rtc, self.rtc, BKP16R, *d),
                17 => write_reg!(rtc, self.rtc, BKP17R, *d),
                18 => write_reg!(rtc, self.rtc, BKP18R, *d),
                19 => write_reg!(rtc, self.rtc, BKP19R, *d),
                20 => write_reg!(rtc, self.rtc, BKP20R, *d),
                21 => write_reg!(rtc, self.rtc, BKP21R, *d),
                22 => write_reg!(rtc, self.rtc, BKP22R, *d),
                23 => write_reg!(rtc, self.rtc, BKP23R, *d),
                24 => write_reg!(rtc, self.rtc, BKP24R, *d),
                25 => write_reg!(rtc, self.rtc, BKP25R, *d),
                26 => write_reg!(rtc, self.rtc, BKP26R, *d),
                27 => write_reg!(rtc, self.rtc, BKP27R, *d),
                28 => write_reg!(rtc, self.rtc, BKP28R, *d),
                29 => write_reg!(rtc, self.rtc, BKP29R, *d),
                30 => write_reg!(rtc, self.rtc, BKP30R, *d),
                31 => write_reg!(rtc, self.rtc, BKP31R, *d),
                _ => (),
            }
        }
    }

    /// Set RTC calibration to match specified actual LSE frequency in Hz.
    pub fn set_calibration(&self, calp: u8, calm: u16) {
        // Unlock RTC registers.
        write_reg!(rtc, self.rtc, WPR, KEY: 0xCA);
        write_reg!(rtc, self.rtc, WPR, KEY: 0x53);

        write_reg!(rtc, self.rtc, CALR, CALP: calp as u32, CALM: calm as u32);

        // Re-lock RTC registers.
        write_reg!(rtc, self.rtc, WPR, KEY: 0);
    }
}

impl core::fmt::Display for DateTime {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "{:02}/{:02}/{:02} {:02}:{:02}:{:02}",
               self.day, self.month, self.year, self.hour, self.minute, self.second)
    }
}

/// Methods for calibrating the LSE given measurements of both LSE and an external GPS reference.
pub struct Calibrator {
    lse: Option<u32>,
    gps: Option<u32>,
}

impl Calibrator {
    pub fn new() -> Self {
        Calibrator { lse: None, gps: None }
    }

    /// Feed a new LSE reading.
    ///
    /// This is a measurement of how many 150MHz timer cycles elapsed over 16384 periods
    /// of the LSE clock (nominally 0.5s and therefore 75E6 cycles).
    pub fn lse_reading(&mut self, lse: u32) {
        self.lse = Some(lse);
    }

    /// Feed a new GPS reading.
    ///
    /// This is a measurement of how many 3MHz timer cycles elapsed over 1 GPS second
    /// (nominally 3E6 cycles).
    pub fn gps_reading(&mut self, gps: u32) {
        self.gps = Some(gps);
    }

    /// Clear any saved readings.
    pub fn clear(&mut self) {
        self.lse = None;
        self.gps = None;
    }

    /// Work out new RTC calibration factors.
    ///
    /// Returns None if no calibration is available or most recent measurements give
    /// an out-of-bounds calibration factor, otherwise `Some((calp, calm))`, where
    /// `calp` is either 0 or 1, and `calm` is in `0..512`.
    pub fn cal(&mut self) -> Option<(u8, u16)> {
        if let Some(gps) = self.gps {
            if let Some(lse) = self.lse {
                self.gps = None;
                self.lse = None;
                let n = (lse as i64) * (1 << 20);
                let m = (gps * 25) as i64;
                let cal = n/m - (1 << 20);
                if cal < -511 || cal > 512 {
                    return None;
                } else if cal > 0 {
                    return Some((1, (cal - 512) as u16));
                } else {
                    return Some((0, (-cal) as u16));
                }
            }
        }
        None
    }
}
