use core::fmt::Write;
use time::{PrimitiveDateTime, OffsetDateTime, Date, Time, Month, Duration, UtcOffset};
use heapless::String;
use embedded_graphics::{
    image::Image,
    mono_font::{ascii::FONT_6X9, ascii::FONT_9X18, MonoTextStyle},
    pixelcolor::Rgb888,
    Pixel,
    prelude::*,
    text::{Alignment, Baseline, Text, TextStyleBuilder},
};
use tinytga::Tga;
use crate::{Name, map::{Map, MAP_NAMES}, menu::{Menu, Category, Setting}};

/// Default URL for QR code if no specific entry is known.
static DEFAULT_URL: &str = "HTTPS://TIMGREIG.CO.UK";

/// Simple date-time representation optimised for grabbing
/// the time components we need without too much extra storage
/// or computation.
#[derive(Copy, Clone, Debug, PartialEq, Default)]
pub struct DateTime {
    year: u16,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    second: u8,
}

impl From<&DateTime> for PrimitiveDateTime {
    fn from(dt: &DateTime) -> PrimitiveDateTime {
        let date = Date::from_calendar_date(
            dt.year as i32, (dt.month as u8).try_into().unwrap(), dt.day as u8).unwrap();
        let time = Time::from_hms(dt.hour, dt.minute, dt.second).unwrap();
        PrimitiveDateTime::new(date, time)
    }
}

impl From<&PrimitiveDateTime> for DateTime {
    fn from(dt: &PrimitiveDateTime) -> DateTime {
        DateTime {
            year: dt.year() as u16, month: dt.month() as u8, day: dt.day(),
            hour: dt.hour(), minute: dt.minute(), second: dt.second(),
        }
    }
}

impl DateTime {
    /// Return a three-letter short name for the current month.
    pub fn month_name_short(&self) -> &'static str {
        match self.month {
            1 => "JAN",
            2 => "FEB",
            3 => "MAR",
            4 => "APR",
            5 => "MAY",
            6 => "JUN",
            7 => "JUL",
            8 => "AUG",
            9 => "SEP",
            10 => "OCT",
            11 => "NOV",
            12 => "DEC",
            _ => "",
        }
    }

    pub fn year(&self) -> u16 {
        self.year
    }

    pub fn month(&self) -> u8 {
        self.month
    }

    pub fn day(&self) -> u8 {
        self.day
    }

    pub fn hour(&self) -> u8 {
        self.hour
    }

    pub fn minute(&self) -> u8 {
        self.minute
    }

    pub fn second(&self) -> u8 {
        self.second
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
enum DisplayType {
    Off,
    Map,
    Qr,
    Jpeg,
}

#[derive(Debug)]
pub struct Clock {
    utc: DateTime,
    local: DateTime,
    map: Option<(Map, Tga<'static, Rgb888>)>,
    gps_status: String<17>,
    frame: u16,
    display_type: DisplayType,
    text_color: Rgb888,
    needs_saving: bool,
    time_set: bool,
    menu: Menu<3, 9>,
}

/// Create the Menu structure used by Clock.
const fn menu() -> Menu<3, 9> {
    Menu::new([
        Category::new(Name::DateTime, [
            Setting::new_onoff(Name::GPSTime, true, true),
            Setting::new_numeric(Name::Year, false, 2000, 2099, 2000),
            Setting::new_numeric(Name::Month, false, 1, 12, 1),
            Setting::new_numeric(Name::Day, false, 1, 31, 1),
            Setting::new_numeric(Name::Hour, false, 0, 23, 0),
            Setting::new_numeric(Name::Minute, false, 0, 59, 0),
            Setting::new_numeric(Name::Second, false, 0, 59, 0),
            Setting::new_onoff(Name::AutomaticDST, true, true),
            Setting::new_numeric(Name::UTCOffset, false, -12, 12, 0),
        ]),
        Category::new(Name::Map, [
            Setting::new_choice(Name::Route, true, 0, MAP_NAMES),
            Setting::new_numeric(Name::RouteDay, true, 0, 999, 0),
            Setting::new_onoff(Name::AnimateRoute, true, true),
            Setting::new_onoff(Name::HourlyImages, true, true),
            Setting::new_disabled(),
            Setting::new_disabled(),
            Setting::new_disabled(),
            Setting::new_disabled(),
            Setting::new_disabled(),
        ]),
        Category::new(Name::Display, [
            Setting::new_numeric(Name::Brightness, true, 0, 10, 10),
            Setting::new_onoff(Name::DimAtNight, true, true),
            Setting::new_numeric(Name::DimBrightness, true, 0, 10, 8),
            Setting::new_numeric(Name::DimStartHour, true, 0, 23, 23),
            Setting::new_numeric(Name::DimEndHour, true, 0, 23, 7),
            Setting::new_disabled(),
            Setting::new_disabled(),
            Setting::new_disabled(),
            Setting::new_disabled(),
        ]),
    ])
}

/// Current version of menu. Increment every time the menu is changed
/// to ensure stale saved menu settings are not incorrectly applied.
const MENU_VERSION: u16 = 2;

impl Clock {
    /// Create a new Clock instance.
    pub fn new() -> Self {
        Clock {
            utc: DateTime::default(),
            local: DateTime::default(),
            map: None,
            gps_status: String::new(),
            frame: 0,
            display_type: DisplayType::Map,
            text_color: Rgb888::WHITE,
            needs_saving: false,
            time_set: false,
            menu: menu(),
        }
    }

    /// Set the date and time in UTC.
    pub fn set_time(&mut self, year: u16, month: u8, day: u8, hour: u8, minute: u8, second: u8) {
        let new = DateTime { year, month, day, hour, minute, second };
        if self.utc != new {
            // Advance map day on day rollover, resetting to 0 at the end.
            if self.time_set && day != self.utc.day {
                if self.map_day() == self.map_days() - 1 {
                    self.set_map_day(0);
                } else {
                    self.set_map_day(self.map_day() + 1);
                }
                self.needs_saving = true;
            }

            // Set new UTC and recompute new local time.
            self.utc = new;
            self.local = self.local_time();

            // Update menu entries for new time.
            let date = self.menu.category_mut(Name::DateTime).unwrap();
            date.setting_set_numeric(Name::Year, year as i16);
            date.setting_set_numeric(Name::Month, month as i16);
            date.setting_set_numeric(Name::Day, day as i16);
            date.setting_set_numeric(Name::Hour, hour as i16);
            date.setting_set_numeric(Name::Minute, minute as i16);
            date.setting_set_numeric(Name::Second, second as i16);

            // Record that the time has been set at least once.
            self.time_set = true;
        }
    }

    /// Set GPS status string to "GPS: Error"
    pub fn set_gps_error(&mut self) {
        self.gps_status.clear();
        self.gps_status.push_str("GPS: Error").ok();
    }

    /// Set the GPS status string to "GPS: No lock" and a brief time since last lock.
    ///
    /// `time_since_last` is seconds since the last good lock, and is reformatted into
    /// minutes, hours, or days.
    pub fn set_gps_no_lock(&mut self, time_since_last: u32) {
        self.gps_status.clear();
        let (count, units) = if time_since_last < 60 {
            (time_since_last, "s")
        } else if time_since_last < 60 * 60 {
            (time_since_last / 60, "m")
        } else if time_since_last < 24 * 60 * 60 {
            (time_since_last / (60 * 60), "h")
        } else {
            (time_since_last / (24 * 60 * 60), "d")
        };
        write!(self.gps_status, "GPS: No lock {}{}", count, units).ok();
    }

    /// Set the GPS status string to "GPS: Good, {num_sv} SVs"
    pub fn set_gps_lock_valid(&mut self, num_sv: u8) {
        self.gps_status.clear();
        write!(self.gps_status, "GPS: Good, {} SVs", num_sv).ok();
    }

    /// Set the GPS status string to "GPS: Wait, {num_sv} SVs"
    pub fn set_gps_lock_invalid(&mut self, num_sv: u8) {
        self.gps_status.clear();
        write!(self.gps_status, "GPS: Wait, {} SVs", num_sv).ok();
    }

    /// Set the GPS status string to "GPS: Unused"
    pub fn set_gps_unused(&mut self) {
        self.gps_status.clear();
        self.gps_status.push_str("GPS: Unused").ok();
    }

    /// Render the clock UI to the provided `DrawTarget`.
    ///
    /// Call `prerender_jpeg()` before this, and if it returns a JPEG,
    /// draw that to the display first, otherwise fill the display with black.
    pub fn render_main<D>(&mut self, display: &mut D)
        where D: DrawTarget<Color = Rgb888>
    {
        self.frame = self.frame.wrapping_add(1);
        match self.display_type {
            DisplayType::Off => (),
            DisplayType::Qr => self.render_main_qr(display),
            DisplayType::Jpeg => self.render_main_datetime(display, self.text_color),
            DisplayType::Map => {
                if self.jpeg_override() {
                    self.render_main_datetime(display, self.text_color);
                } else {
                    self.render_main_clock(display);
                }
            }
        }
    }

    /// Render the sub-display with status or menu.
    pub fn render_sub<D>(&self, display: &mut D)
        where D: DrawTarget<Color = Rgb888>
    {
        if self.menu.active() {
            self.render_menu(display);
        } else {
            self.render_status(display);
        }
    }

    /// Call when the BACK key is pressed.
    pub fn key_back(&mut self) {
        self.menu.back();
    }

    /// Call when the QR key is pressed.
    pub fn key_qr(&mut self) {
        self.display_type = match self.display_type {
            DisplayType::Off => DisplayType::Off,
            DisplayType::Map => DisplayType::Qr,
            DisplayType::Qr => DisplayType::Map,
            DisplayType::Jpeg => DisplayType::Qr,
        }
    }

    /// Call when the DISPLAY key is pressed.
    pub fn key_display(&mut self) {
        self.display_type = match self.display_type {
            DisplayType::Off => DisplayType::Map,
            DisplayType::Map => DisplayType::Jpeg,
            DisplayType::Jpeg => DisplayType::Off,
            DisplayType::Qr => DisplayType::Off,
        }
    }

    /// Call when the ENTER key is pressed.
    pub fn key_enter(&mut self) {
        self.menu.enter();
    }

    /// Call when the LEFT key is pressed.
    pub fn key_left(&mut self) {
        if self.menu.active() && self.menu.dec() {
            self.process_menu_update();
        }
    }

    /// Call when the RIGHT key is pressed.
    pub fn key_right(&mut self) {
        if self.menu.active() && self.menu.inc() {
            self.process_menu_update();
        }
    }

    /// Check if the user changed the time in the most recent menu interaction.
    ///
    /// This method compares the menu's value which may change with calls to `key_dec()`
    /// or `key_inc()` with the Clock's setting which may change with calls to `set_date()`
    /// and `set_time()`, so check it between calling those methods.
    pub fn time_changed(&self) -> bool {
        let (year, month, day, hour, minute, second) = self.menu_time();
        self.utc != DateTime { year, month, day, hour, minute, second }
    }

    /// Get the current UTC (year, month, day, hour, minute, second) set in the menu.
    pub fn menu_time(&self) -> (u16, u8, u8, u8, u8, u8) {
        let dt = self.menu.category(Name::DateTime).unwrap();
        (
            (dt.setting_numeric(Name::Year).unwrap()) as u16,
            dt.setting_numeric(Name::Month).unwrap() as u8,
            dt.setting_numeric(Name::Day).unwrap() as u8,
            dt.setting_numeric(Name::Hour).unwrap() as u8,
            dt.setting_numeric(Name::Minute).unwrap() as u8,
            dt.setting_numeric(Name::Second).unwrap() as u8,
        )
    }

    /// Get current desired brightness.
    ///
    /// If the display has been toggled off using the DISPLAY button,
    /// returns 0. Otherwise, brightness is determined from menu settings
    /// and current time, depending on user-set brightness and dim-at-night.
    ///
    /// Returns a number 0 to 10.
    pub fn brightness(&self) -> u8 {
        if self.display_type == DisplayType::Off {
            return 0;
        }

        let disp = self.menu.category(Name::Display).unwrap();
        let brightness = disp.setting_numeric(Name::Brightness).unwrap();
        if disp.setting_onoff(Name::DimAtNight).unwrap() {
            let DateTime { hour, .. } = self.local;
            let start = disp.setting_numeric(Name::DimStartHour).unwrap() as u8;
            let end = disp.setting_numeric(Name::DimEndHour).unwrap() as u8;
            if hour >= start || hour < end {
                disp.setting_numeric(Name::DimBrightness).unwrap() as u8
            } else {
                brightness as u8
            }
        } else {
            brightness as u8
        }
    }

    /// Returns whether the user wants to use GPS to update the time.
    ///
    /// If true, then when available, set the system time using GPS,
    /// and do not use `time_changed()` or `menu_time()` to change it.
    ///
    /// Otherwise, let time advance normally, and respect user changes.
    pub fn use_gps_time(&self) -> bool {
        self.menu.category(Name::DateTime).unwrap().setting_onoff(Name::GPSTime).unwrap()
    }

    /// Return whether the clock settings have changed and need saving since they
    /// were last serialised.
    ///
    /// Cleared by calling `serialise()`.
    pub fn needs_saving(&self) -> bool {
        self.needs_saving
    }

    /// If a JPEG image should be displayed, returns a &[u8] to the JPEG data.
    /// The data should be rendered to the main display before calling `render_main()`,
    /// which will then draw just the clock elements on top.
    pub fn prerender_jpeg(&self) -> Option<&'static [u8]> {
        if self.display_type == DisplayType::Jpeg || self.jpeg_override() {
            self.map.map(|(m, _)| m.image(self.map_day())).flatten()
        } else {
            None
        }
    }

    /// Serialise state to &[u32], which must be large enough to hold all used settings.
    ///
    /// This will never exceed 32 u32s.
    ///
    /// State includes all menu settings.
    pub fn serialise(&mut self, data: &mut [u32]) {
        self.needs_saving = false;
        // NOTE(unsafe): Menu serialises to u16 and we'd like to pack those into our u32.
        // NOTE(unsafe): We make sure to not use the incoming slice after making this new one.
        let data = unsafe {
            core::slice::from_raw_parts_mut(
                data.as_mut_ptr() as *mut u16,
                data.len() * 2,
            )
        };
        data[0] = MENU_VERSION;
        self.menu.serialise(&mut data[2..]);
        data[1] = crc16(&data[2..]);
    }

    /// Deserialise state from a &[u16] which was previously serialised to.
    pub fn deserialise(&mut self, data: &[u32]) {
        // NOTE(unsafe): Menu serialises from u16 and we'd like to pack those into our u32.
        // NOTE(unsafe): We make sure to not use the incoming slice after making this new one.
        let data = unsafe {
            core::slice::from_raw_parts(
                data.as_ptr() as *const u16,
                data.len() * 2,
            )
        };
        let crc = crc16(&data[2..]);
        if data[0] == MENU_VERSION && data[1] == crc {
            self.menu.deserialise(&data[2..]);
            let map_day = self.map_day();
            self.process_menu_update();
            self.set_map_day(map_day);
        }
    }
}

impl Clock {
    /// Select which map to render.
    fn set_map(&mut self, map: Map) {
        if let Some(data) = map.background() {
            if let Ok(tga) = Tga::from_slice(data) {
                self.map = Some((map, tga));
                let map_menu = self.menu.category_mut(Name::Map).unwrap();
                map_menu.setting_set_choice(Name::Route, map.name());
                map_menu.setting_set_max(Name::RouteDay, map.days() as i16 - 1);
                map_menu.setting_set_numeric(Name::RouteDay, 0);
            }
        }
    }

    /// Get current map day from menu settings.
    fn map_day(&self) -> u8 {
        let menu = self.menu.category(Name::Map).unwrap();
        menu.setting_numeric(Name::RouteDay).unwrap() as u8
    }

    /// Set the day-of-walk.
    fn set_map_day(&mut self, day: u8) {
        if let Some((map, _)) = self.map {
            if (day as usize) < map.days() {
                let map_menu = self.menu.category_mut(Name::Map).unwrap();
                map_menu.setting_set_numeric(Name::RouteDay, day as i16);
            }
        }
    }

    /// Get maximum number of days in current map.
    fn map_days(&self) -> u8 {
        if let Some((map, _)) = self.map {
            map.days() as u8
        } else {
            0
        }
    }

    /// Get the current local date/time.
    ///
    /// Offsets the internal UTC time by the current UTC offset,
    /// which may be determined automatically.
    ///
    /// Returns (year, month, day, hour, minute, second).
    fn local_time(&self) -> DateTime {
        let DateTime { year, month, day, hour, minute, second } = self.utc;
        let date = Date::from_calendar_date(
            year as i32, month.try_into().unwrap(), day).unwrap();
        let time = Time::from_hms(hour, minute, second).unwrap();
        let utc = PrimitiveDateTime::new(date, time).assume_utc();
        let local = utc.to_offset(self.utc_offset());
        DateTime {
            year: local.year() as u16, month: local.month() as u8, day: local.day(),
            hour: local.hour(), minute: local.minute(), second: local.second(),
        }
    }

    /// Get our current UTC offset at our current UTC time.
    fn utc_offset(&self) -> UtcOffset
    {
        let dt = self.menu.category(Name::DateTime).unwrap();
        if dt.setting_onoff(Name::AutomaticDST).unwrap() {
            let DateTime { year, month, day, hour, minute, second } = self.utc;
            let date = Date::from_calendar_date(
                year as i32, month.try_into().unwrap(), day).unwrap();
            let time = Time::from_hms(hour, minute, second).unwrap();
            let utc = PrimitiveDateTime::new(date, time).assume_utc();
            Self::automatic_dst(&utc)
        } else {
            let off = dt.setting_numeric(Name::UTCOffset).unwrap() as i8;
            UtcOffset::from_hms(off, 0, 0).unwrap()
        }
    }

    /// Compute UK UTC offset at given date/time.
    ///
    /// Returns UTC+1 between 01:00 UTC on the last Sunday in March
    /// and 01:00 UTC on the last Sunday in October, and UTC+0 otherwise.
    fn automatic_dst(time: &OffsetDateTime) -> UtcOffset {
        // Automatic UK DST.
        // UTC+1 after 1am UTC on the last Sunday in March,
        // UTC+0 after 1am UTC on the last Sunday in October.
        let utc = time.to_offset(UtcOffset::UTC);

        // Find last Sunday in March.
        let march31 = Date::from_calendar_date(utc.year(), Month::March, 31).unwrap();
        let days = march31.weekday().number_days_from_sunday();
        let last_sun = march31 - Duration::days(days as i64);
        let start = last_sun.with_hms(1, 0, 0).unwrap().assume_utc();

        // Find last Sunday in October.
        let oct31 = Date::from_calendar_date(utc.year(), Month::October, 31).unwrap();
        let days = oct31.weekday().number_days_from_sunday();
        let last_sun = oct31 - Duration::days(days as i64);
        let end = last_sun.with_hms(1, 0, 0).unwrap().assume_utc();

        // Check if we're inside UK DST.
        if utc >= start && utc <= end {
            UtcOffset::from_hms(1, 0, 0).unwrap()
        } else {
            UtcOffset::UTC
        }
    }

    /// Render main clock, either with a route or just a plain date/time display.
    fn render_main_clock<D>(&self, display: &mut D)
        where D: DrawTarget<Color = Rgb888>
    {
        if let Some((map, ref bg)) = self.map {
            let img = Image::new(bg, Point::zero());
            img.draw(display).ok();
            let map_menu = self.menu.category(Name::Map).unwrap();
            let animate = map_menu.setting_onoff(Name::AnimateRoute).unwrap();
            map.render(display, &self.local, self.frame, self.map_day(), animate);
            self.render_main_datetime(display, self.text_color);
        } else {
            self.render_big_datetime(display, self.text_color);
        }
    }

    /// Render a QR code for the current day, if known.
    fn render_main_qr<D>(&self, display: &mut D)
        where D: DrawTarget<Color = Rgb888>
    {
        let mut outbuffer = [0u8; 128];
        let mut tmpbuffer = [0u8; 128];
        use crate::qr::{QrCode, QrCodeEcc, Version};
        let url = self.map.map(|(m, _)| m.url(self.map_day())).flatten().unwrap_or(DEFAULT_URL);
        let qr = QrCode::encode_text(
            url, &mut tmpbuffer[..], &mut outbuffer[..], QrCodeEcc::Low,
            Version::new(3), Version::new(3), None, true);
        if let Ok(qr) = qr {
            for x in 0..37 {
                for y in 0..37 {
                    Pixel(Point::new(13 + x, 13 + y), Rgb888::WHITE).draw(display).ok();
                }
            }
            for y in 0..qr.size() {
                for x in 0..qr.size() {
                    if qr.get_module(x, y) {
                        let x = 13 + 4 + x as i32;
                        let y = 13 + 4 + y as i32;
                        Pixel(Point::new(x, y), Rgb888::BLACK).draw(display).ok();
                    }
                }
            }
        }
    }

    /// Render the date and time on the top-left and top-right of the display.
    fn render_main_datetime<D>(&self, display: &mut D, color: Rgb888)
        where D: DrawTarget<Color = Rgb888>
    {
        let font = MonoTextStyle::new(&FONT_6X9, color);
        let tr_style = TextStyleBuilder::new()
            .alignment(Alignment::Right)
            .baseline(Baseline::Top)
            .build();
        let tl_style = TextStyleBuilder::new()
            .alignment(Alignment::Left)
            .baseline(Baseline::Top)
            .build();
        let mut s: String<5> = String::new();

        let DateTime { month, day, hour, minute, second, .. } = self.local;

        // Date
        write!(&mut s, "{:2}/{}", day, month).ok();
        Text::with_text_style(&s, Point::new(0, 0), font, tl_style).draw(display).ok();
        s.clear();

        // Time
        write!(&mut s, "{:02}", hour).ok();
        Text::with_text_style(&s, Point::new(49, 0), font, tr_style).draw(display).ok();
        s.clear();
        if second % 2 == 0 {
            Text::with_text_style(":",  Point::new(53, 0), font, tr_style).draw(display).ok();
        }
        write!(&mut s, "{:02}", minute).ok();
        Text::with_text_style(&s, Point::new(63, 0), font, tr_style).draw(display).ok();
        s.clear();

        self.render_greetings(display);
    }

    /// Render any special greetings for current date.
    fn render_greetings<D>(&self, display: &mut D) where D: DrawTarget<Color = Rgb888> {
        let font = MonoTextStyle::new(&FONT_6X9, Rgb888::MAGENTA);
        let style = TextStyleBuilder::new()
            .alignment(Alignment::Left)
            .baseline(Baseline::Top)
            .build();

        if self.local.month() == 12 && self.local.day() == 31 {
            Text::with_text_style("Happy", Point::new(0, 10), font, style).draw(display).ok();
            Text::with_text_style("NYE!", Point::new(12, 44), font, style).draw(display).ok();
        }
    }

    /// Render the date and time large and central.
    fn render_big_datetime<D>(&self, display: &mut D, color: Rgb888)
        where D: DrawTarget<Color = Rgb888>
    {
        let font = MonoTextStyle::new(&FONT_9X18, color);
        let style = TextStyleBuilder::new()
            .alignment(Alignment::Center)
            .baseline(Baseline::Top)
            .build();
        let mut s: String<7> = String::new();

        let DateTime { day, hour, minute, second, .. } = self.local;
        let month = self.local.month_name_short();

        // Date
        write!(&mut s, "{} {}", day, month).ok();
        Text::with_text_style(&s, Point::new(32, 12), font, style).draw(display).ok();
        s.clear();

        // Time
        if second % 2 == 0 {
            write!(&mut s, "{:02}:{:02}", hour, minute).ok();
        } else {
            write!(&mut s, "{:02} {:02}", hour, minute).ok();
        }
        Text::with_text_style(&s, Point::new(32, 32), font, style).draw(display).ok();
    }

    /// Render the status screen.
    /// Shows current date/time, GPS status, and instructions to enter menu.
    fn render_status<D>(&self, display: &mut D)
        where D: DrawTarget<Color = Rgb888>
    {
        let font = MonoTextStyle::new(&FONT_9X18, Rgb888::WHITE);
        let style = TextStyleBuilder::new()
            .alignment(Alignment::Left)
            .baseline(Baseline::Top)
            .build();
        let mut s: String<17> = String::new();

        let DateTime { year, month, day, hour, minute, second } = self.local;

        write!(&mut s, "{:02}/{:02}/{:02} {:02}:{:02}:{:02}",
               day, month, year - 2000, hour, minute, second).ok();
        Text::with_text_style(&s, Point::new(0, 0), font, style).draw(display).ok();
        s.clear();

        Text::with_text_style(&self.gps_status, Point::new(0, 20), font, style).draw(display).ok();

        write!(&mut s, "   Press ENTER").ok();
        Text::with_text_style(&s, Point::new(0, 40), font, style).draw(display).ok();
        s.clear();

        write!(&mut s, "   to open menu").ok();
        Text::with_text_style(&s, Point::new(0, 60), font, style).draw(display).ok();
        s.clear();
    }

    /// Render the menu.
    /// Draws the current category, setting, and value, with left/right arrows
    /// indicating which is currently being changed by the left/right buttons.
    fn render_menu<D>(&self, display: &mut D)
        where D: DrawTarget<Color = Rgb888>
    {
        let font = MonoTextStyle::new(&FONT_9X18, Rgb888::WHITE);
        let style = TextStyleBuilder::new()
            .alignment(Alignment::Left)
            .baseline(Baseline::Top)
            .build();
        let mut s: String<17> = String::new();

        write!(&mut s, "MENU").ok();
        Text::with_text_style(&s, Point::new(58, 0), font, style).draw(display).ok();
        s.clear();

        if !self.menu.category_selected() {
            write!(&mut s, "< {:^13} >", self.menu.category_name().into_str()).ok();
        } else {
            write!(&mut s, "  {:^13}", self.menu.category_name().into_str()).ok();
        }
        Text::with_text_style(&s, Point::new(0, 20), font, style).draw(display).ok();
        s.clear();

        if self.menu.category_selected() && !self.menu.setting_selected() {
            write!(&mut s, "< {:^14}>", self.menu.setting_name().into_str()).ok();
        } else {
            write!(&mut s, "  {:^14}", self.menu.setting_name().into_str()).ok();
        }
        Text::with_text_style(&s, Point::new(0, 40), font, style).draw(display).ok();
        s.clear();

        let mut v: String<14> = String::new();
        self.menu.render_value(&mut v).ok();
        if self.menu.category_selected() && self.menu.setting_selected() {
            write!(&mut s, "< {:^14}>", v).ok();
        } else {
            write!(&mut s, "  {:^14}", v).ok();
        }
        Text::with_text_style(&s, Point::new(0, 60), font, style).draw(display).ok();
    }

    /// Update internal menu state after a value is changed.
    /// Enables/disables fields as appropriate.
    fn process_menu_update(&mut self) {
        // Enable/disable date/time settings as appropriate.
        let date = self.menu.category_mut(Name::DateTime).unwrap();
        let gps = date.setting_onoff(Name::GPSTime).unwrap();
        date.setting_set_enabled(Name::Year, !gps);
        date.setting_set_enabled(Name::Month, !gps);
        date.setting_set_enabled(Name::Day, !gps);
        date.setting_set_enabled(Name::Hour, !gps);
        date.setting_set_enabled(Name::Minute, !gps);
        date.setting_set_enabled(Name::Second, !gps);
        let dst = date.setting_onoff(Name::AutomaticDST).unwrap();
        date.setting_set_enabled(Name::UTCOffset, !dst);

        // Limit days in month for current month.
        let year = date.setting_numeric(Name::Year).unwrap() as i32;
        let month = (date.setting_numeric(Name::Month).unwrap() as u8).try_into().unwrap();
        date.setting_set_max(Name::Day, time::util::days_in_year_month(year, month) as i16);

        // Recompute local time immediately when automatic DST is disabled.
        if !dst {
            self.local = self.local_time();
        }

        // Enable/disable night-time dimming settings as appropriate.
        let disp = self.menu.category_mut(Name::Display).unwrap();
        let dim = disp.setting_onoff(Name::DimAtNight).unwrap();
        disp.setting_set_enabled(Name::DimBrightness, dim);
        disp.setting_set_enabled(Name::DimStartHour, dim);
        disp.setting_set_enabled(Name::DimEndHour, dim);

        // Restore map and map-day, also setting map-day maximum value in `set_map()`.
        let map_menu = self.menu.category(Name::Map).unwrap();
        let map_choice = map_menu.setting_choice(Name::Route).unwrap();
        if let Ok(map) = Map::try_from(map_choice) {
            if let Some((current_map, _)) = self.map {
                if map_choice != current_map.name() {
                    self.set_map(map);
                }
            } else {
                self.set_map(map);
            }
        } else {
            self.map = None;
        }

        // Record that settings may have changed and should be saved.
        self.needs_saving = true;
    }

    /// Return whether to override map mode with jpeg mode briefly.
    ///
    /// Only returns true if the "Hourly images" setting is enabled,
    /// the display is currently on map mode, there is an image for
    /// this day, and it's in the first minute of the hour.
    fn jpeg_override(&self) -> bool {
        let map_menu = self.menu.category(Name::Map).unwrap();
        let hourly_images = map_menu.setting_onoff(Name::HourlyImages).unwrap();
        let on_map = self.display_type == DisplayType::Map;
        let got_image = self.map.map(|(m, _)| m.image(self.map_day())).flatten().is_some();
        let first_minute = self.local.minute == 0;

        hourly_images && on_map && got_image && first_minute
    }
}

/// Compute a CRC-16 over 16-bit input data.
///
/// Uses the common CRC-16 polynomial 0x1021 with model parameters:
///
/// `width=16 poly=0x1021 init=0xffff refin=false refout=false xorout=0xffff`
///
/// The input 16-bit words are processed as though they were a stream of bytes,
/// most-significant-byte first.
///
/// In other words, the input `&[0x0123, 0x4567, 0x89ab, 0xcdef]` is equivalent
/// to the 8-bit input `&[0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef]`.
fn crc16(data: &[u16]) -> u16 {
    const POLY: u16 = 0x1021;
    let mut crc: u16 = 0xFFFF;
    for word in data.iter() {
        crc ^= word;
        for _ in 0..16 {
            if (crc & 0x8000) != 0 {
                crc = (crc << 1) ^ POLY;
            } else {
                crc <<= 1;
            }
        }
    }
    crc ^ 0xFFFF
}
