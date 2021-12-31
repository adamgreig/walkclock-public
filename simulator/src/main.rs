use std::fs::File;
use std::io::prelude::*;
use time::{PrimitiveDateTime, OffsetDateTime, Date, Time, Duration};
use embedded_graphics::{prelude::*, pixelcolor::Rgb888};
use embedded_graphics_simulator::{
    OutputSettings, SimulatorDisplay, Window, SimulatorEvent, sdl2::Keycode,
};
use walkclock::Clock;

pub fn main() {
    let mut main_display = SimulatorDisplay::new(Size::new(64, 64));
    let main_settings = OutputSettings { scale: 8, pixel_spacing: 2, ..Default::default() };
    let mut main_window = Window::new("ClockSim", &main_settings);

    let mut sub_display = SimulatorDisplay::new(Size::new(160, 80));
    let sub_settings = OutputSettings { scale: 1, pixel_spacing: 0, ..Default::default() };
    let mut sub_window = Window::new("ClockSim Control", &sub_settings);

    let mut clock = Clock::new();
    clock.set_gps_unused();

    if let Ok(mut file) = File::open("clock_state.bin") {
        let mut settings = [0u32; 32];
        for word in settings.iter_mut() {
            let mut buf = [0u8; 4];
            file.read_exact(&mut buf[..]).expect("Error reading state file");
            *word = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]);
        }
        clock.deserialise(&settings[..]);
    }

    let mut wallclock: OffsetDateTime = OffsetDateTime::now_utc();

    'outer: loop {
        let t0 = std::time::Instant::now();

        if clock.use_gps_time() {
            wallclock = OffsetDateTime::now_utc()
        }
        clock.set_time(
            wallclock.year() as u16, wallclock.month() as u8, wallclock.day() as u8,
            wallclock.hour() as u8, wallclock.minute() as u8, wallclock.second() as u8,
        );

        if let Some(jpeg) = clock.prerender_jpeg() {
            let mut decoder = jpeg_decoder::Decoder::new(jpeg);
            let pixels = decoder.decode().expect("Failed to parse JPEG");
            for x in 0..64 {
                for y in 0..64 {
                    let r = pixels[y*64*3 + x*3 + 0];
                    let g = pixels[y*64*3 + x*3 + 1];
                    let b = pixels[y*64*3 + x*3 + 2];
                    Pixel(Point::new(x as i32, y as i32), Rgb888::new(r, g, b))
                        .draw(&mut main_display).ok();
                }
            }
        } else {
            main_display.clear(Rgb888::new(0, 0, 0)).unwrap();
        }
        clock.render_main(&mut main_display);
        main_window.update(&main_display);

        sub_display.clear(Rgb888::new(0, 0, 0)).unwrap();
        clock.render_sub(&mut sub_display);
        sub_window.update(&sub_display);

        for event in main_window.events() {
            match event {
                SimulatorEvent::Quit => break 'outer,
                SimulatorEvent::KeyDown { keycode, .. } => match keycode {
                        Keycode::Escape => break 'outer,

                        Keycode::Q => clock.key_back(),
                        Keycode::W => clock.key_qr(),
                        Keycode::E => clock.key_display(),
                        Keycode::A => clock.key_enter(),
                        Keycode::S => clock.key_left(),
                        Keycode::D => clock.key_right(),

                        Keycode::Return => clock.key_enter(),
                        Keycode::Backspace => clock.key_back(),
                        Keycode::Up => clock.key_back(),
                        Keycode::Down => clock.key_enter(),
                        Keycode::Left => clock.key_left(),
                        Keycode::Right => clock.key_right(),

                        _ => (),
                },
                _ => (),
            }
        }

        if clock.time_changed() {
            let (year, month, day, hour, minute, second) = clock.menu_time();
            let date = Date::from_calendar_date(
                year as i32, month.try_into().unwrap(), day as u8).unwrap();
            let time = Time::from_hms(hour as u8, minute as u8, second as u8).unwrap();
            wallclock = PrimitiveDateTime::new(date, time).assume_utc();
        }

        if clock.needs_saving() {
            let mut settings = [0u32; 32];
            clock.serialise(&mut settings[..]);
            let mut file = File::create("clock_state.bin").expect("Error opening file");
            for word in settings {
                file.write_all(&word.to_le_bytes()).expect("Error writing file");
            }
        }

        // Copy firmware's 20Hz display update rate.
        let elapsed = t0.elapsed();
        if elapsed < std::time::Duration::from_millis(50) {
            std::thread::sleep(std::time::Duration::from_millis(50) - elapsed);
        }
        wallclock += Duration::milliseconds(50);
    }
}
