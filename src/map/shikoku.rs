use core::fmt::Write;
use heapless::String;
use embedded_graphics::{
    mono_font::{ascii::FONT_6X9, MonoTextStyle},
    pixelcolor::Rgb888,
    prelude::*,
    text::{Alignment, Baseline, Text, TextStyleBuilder},
};
use crate::clock::DateTime;

pub static IMAGE: &[u8] = include_bytes!("../../artwork/shikoku/shikoku_base.tga");

static ROUTE: [(u8, u8); 292] = [
    (58, 15), (59, 15), (59, 16), (58, 16), (57, 16), (56, 16), (56, 17), (55, 16),
    (55, 17), (54, 18), (54, 17), (55, 18), (53, 18), (53, 19), (52, 19), (52, 20),
    (53, 20), (52, 21), (53, 21), (53, 22), (52, 22), (52, 23), (52, 24), (53, 24),
    (53, 23), (54, 23), (54, 24), (55, 24), (56, 24), (54, 22), (55, 22), (55, 23),
    (56, 22), (56, 23), (57, 23), (57, 24), (58, 24), (58, 23), (59, 23), (59, 24),
    (60, 24), (60, 25), (59, 25), (59, 26), (59, 27), (59, 28), (60, 28), (58, 28),
    (57, 28), (57, 29), (56, 28), (56, 29), (56, 30), (56, 31), (55, 31), (54, 31),
    (54, 32), (53, 32), (53, 33), (53, 34), (52, 34), (52, 35), (51, 35), (51, 36),
    (51, 37), (50, 37), (50, 38), (50, 39), (50, 40), (50, 41), (50, 42), (49, 42),
    (49, 41), (48, 40), (47, 40), (47, 39), (47, 38), (46, 38), (46, 37), (45, 37),
    (45, 36), (44, 36), (44, 35), (43, 35), (42, 35), (41, 35), (40, 35), (39, 35),
    (39, 34), (38, 34), (37, 33), (36, 33), (36, 32), (37, 32), (37, 31), (36, 31),
    (35, 31), (34, 31), (33, 31), (33, 32), (34, 32), (35, 32), (35, 33), (34, 33),
    (33, 33), (32, 34), (32, 35), (31, 35), (31, 36), (32, 36), (32, 37), (31, 37),
    (30, 37), (31, 38), (30, 38), (30, 39), (30, 40), (29, 40), (29, 41), (28, 41),
    (28, 42), (28, 43), (27, 43), (26, 44), (25, 44), (26, 45), (25, 45), (24, 46),
    (24, 47), (24, 48), (24, 49), (23, 49), (23, 50), (22, 50), (22, 51), (22, 52),
    (22, 53), (22, 54), (22, 55), (22, 56), (21, 56), (22, 57), (21, 57), (19, 57),
    (18, 57), (18, 56), (17, 56), (17, 55), (17, 54), (16, 54), (15, 53), (14, 53),
    (15, 52), (14, 52), (13, 51), (13, 50), (12, 50), (11, 50), (11, 49), (11, 48),
    (11, 47), (11, 46), (12, 46), (12, 45), (13, 45), (13, 44), (13, 43), (12, 43),
    (12, 42), (11, 42), (11, 41), (10, 41), (10, 40), (11, 40), (12, 39), (12, 38),
    (13, 38), (13, 37), (13, 36), (14, 36), (14, 35), (15, 35), (15, 34), (16, 34),
    (16, 33), (17, 33), (17, 32), (17, 31), (18, 31), (18, 30), (17, 30), (17, 29),
    (16, 29), (16, 28), (17, 27), (17, 26), (17, 25), (18, 26), (18, 27), (19, 27),
    (19, 26), (20, 26), (20, 27), (21, 27), (21, 26), (21, 25), (20, 25), (19, 25),
    (18, 25), (17, 23), (18, 23), (19, 23), (18, 22), (19, 22), (19, 21), (20, 21),
    (21, 21), (21, 20), (22, 20), (22, 21), (23, 22), (23, 23), (24, 23), (24, 24),
    (24, 25), (25, 24), (26, 24), (27, 24), (28, 24), (28, 25), (29, 24), (29, 23),
    (30, 23), (30, 24), (31, 25), (32, 25), (32, 24), (32, 23), (33, 23), (34, 23),
    (35, 23), (35, 22), (36, 22), (36, 21), (37, 21), (37, 20), (38, 20), (38, 19),
    (37, 18), (38, 18), (38, 17), (40, 18), (40, 17), (41, 17), (41, 18), (42, 18),
    (42, 17), (43, 17), (43, 18), (44, 18), (44, 17), (44, 16), (43, 16), (42, 16),
    (41, 16), (40, 16), (40, 14), (41, 14), (41, 13), (42, 13), (42, 12), (43, 12),
    (44, 11), (44, 12), (44, 13), (45, 13), (46, 12), (46, 11), (47, 11), (47, 12),
    (48, 13), (49, 13), (50, 13), (51, 12), (51, 13), (51, 14), (52, 14), (52, 15),
    (53, 15), (52, 16), (52, 17), (53, 17),
];

pub static DAYS: [u16; 52] = [
    0, 6, 15, 18, 22, 29, 41, 47, 53, 56, 58, 62, 68, 78, 82, 90, 98, 105, 112, 119, 121, 127, 133,
    137, 143, 145, 150, 158, 162, 174, 182, 187, 191, 201, 205, 209, 214, 220, 226, 233, 237, 241,
    243, 247, 261, 272, 276, 278, 289, 291, 292, 292
];

static TEMPLES: [(u8, u8); 88] = [
    (58, 15), (59, 16), (57, 16), (56, 17), (55, 16), (54, 17), (55, 18), (53, 18),
    (52, 19), (53, 20), (52, 21), (52, 23), (56, 24), (57, 23), (58, 24), (59, 23),
    (60, 24), (59, 27), (60, 28), (57, 29), (56, 28), (56, 31), (54, 31), (50, 42),
    (48, 40), (47, 39), (44, 36), (39, 34), (36, 31), (34, 31), (33, 33), (32, 35),
    (31, 36), (32, 37), (30, 37), (30, 40), (26, 45), (22, 57), (15, 53), (11, 50),
    (11, 42), (10, 41), (11, 40), (17, 33), (17, 31), (18, 30), (17, 29), (16, 28),
    (17, 27), (17, 25), (18, 26), (17, 23), (18, 22), (21, 20), (22, 21), (23, 22),
    (24, 23), (24, 25), (25, 24), (28, 25), (29, 24), (30, 23), (31, 25), (32, 24),
    (35, 22), (36, 21), (37, 20), (38, 19), (37, 18), (38, 17), (40, 17), (41, 18),
    (42, 17), (43, 18), (44, 17), (43, 16), (41, 16), (41, 14), (42, 13), (43, 12),
    (44, 11), (44, 13), (46, 12), (47, 11), (51, 12), (51, 14), (53, 15), (52, 16),
];

static N_TEMPLES: [u8; 52] = [
    0, 3, 9, 11, 12, 13, 17, 19, 21, 23, 23, 23, 23, 26, 27, 28, 30, 31, 34, 36, 36, 37, 37, 37,
    38, 38, 38, 40, 40, 43, 43, 44, 46, 51, 51, 51, 53, 55, 59, 62, 64, 64, 65, 67, 75, 80, 82, 83,
    87, 88, 88, 88,
];

const TEMPLE_COLOR: Rgb888 = Rgb888::new(200, 50, 50);

pub static URLS: &[&str] = &[
    "HTTPS://TIMGREIG.CO.UK/2019/09/12/DAY-T",
    "HTTPS://TIMGREIG.CO.UK/2019/09/13/DAY-1",
    "HTTPS://TIMGREIG.CO.UK/2019/09/14/DAY-2",
    "HTTPS://TIMGREIG.CO.UK/2019/09/15/DAY-3",
    "HTTPS://TIMGREIG.CO.UK/2019/09/16/DAY-4",
    "HTTPS://TIMGREIG.CO.UK/2019/09/17/DAY-5",
    "HTTPS://TIMGREIG.CO.UK/2019/09/18/DAY-6",
    "HTTPS://TIMGREIG.CO.UK/2019/09/19/DAY-7",
    "HTTPS://TIMGREIG.CO.UK/2019/09/20/DAY-8",
    "HTTPS://TIMGREIG.CO.UK/2019/09/21/DAY-9",
    "HTTPS://TIMGREIG.CO.UK/2019/09/22/DAY-10",
    "HTTPS://TIMGREIG.CO.UK/2019/09/23/DAY-11",
    "HTTPS://TIMGREIG.CO.UK/2019/09/24/DAY-12",
    "HTTPS://TIMGREIG.CO.UK/2019/09/25/DAY-13",
    "HTTPS://TIMGREIG.CO.UK/2019/09/26/DAY-14",
    "HTTPS://TIMGREIG.CO.UK/2019/09/27/DAY-15",
    "HTTPS://TIMGREIG.CO.UK/2019/09/28/DAY-16",
    "HTTPS://TIMGREIG.CO.UK/2019/09/29/DAY-17",
    "HTTPS://TIMGREIG.CO.UK/2019/09/30/DAY-18",
    "HTTPS://TIMGREIG.CO.UK/2019/10/01/DAY-19",
    "HTTPS://TIMGREIG.CO.UK/2019/10/02/DAY-20",
    "HTTPS://TIMGREIG.CO.UK/2019/10/03/DAY-21",
    "HTTPS://TIMGREIG.CO.UK/2019/10/04/DAY-22",
    "HTTPS://TIMGREIG.CO.UK/2019/10/05/DAY-23",
    "HTTPS://TIMGREIG.CO.UK/2019/10/06/DAY-24",
    "HTTPS://TIMGREIG.CO.UK/2019/10/07/DAY-25",
    "HTTPS://TIMGREIG.CO.UK/2019/10/08/DAY-26",
    "HTTPS://TIMGREIG.CO.UK/2019/10/09/DAY-27",
    "HTTPS://TIMGREIG.CO.UK/2019/10/10/DAY-28",
    "HTTPS://TIMGREIG.CO.UK/2019/10/11/DAY-29",
    "HTTPS://TIMGREIG.CO.UK/2019/10/12/DAY-30",
    "HTTPS://TIMGREIG.CO.UK/2019/10/13/DAY-31",
    "HTTPS://TIMGREIG.CO.UK/2019/10/14/DAY-32",
    "HTTPS://TIMGREIG.CO.UK/2019/10/15/DAY-33",
    "HTTPS://TIMGREIG.CO.UK/2019/10/16/DAY-34",
    "HTTPS://TIMGREIG.CO.UK/2019/10/17/DAY-35",
    "HTTPS://TIMGREIG.CO.UK/2019/10/18/DAY-36",
    "HTTPS://TIMGREIG.CO.UK/2019/10/19/DAY-37",
    "HTTPS://TIMGREIG.CO.UK/2019/10/20/DAY-38",
    "HTTPS://TIMGREIG.CO.UK/2019/10/21/DAY-39",
    "HTTPS://TIMGREIG.CO.UK/2019/10/22/DAY-40",
    "HTTPS://TIMGREIG.CO.UK/2019/10/23/DAY-41",
    "HTTPS://TIMGREIG.CO.UK/2019/10/24/DAY-42",
    "HTTPS://TIMGREIG.CO.UK/2019/10/25/DAY-43",
    "HTTPS://TIMGREIG.CO.UK/2019/10/26/DAY-44",
    "HTTPS://TIMGREIG.CO.UK/2019/10/27/DAY-45",
    "HTTPS://TIMGREIG.CO.UK/2019/10/28/DAY-46",
    "HTTPS://TIMGREIG.CO.UK/2019/10/29/DAY-47",
    "HTTPS://TIMGREIG.CO.UK/2019/10/30/DAY-48",
    "HTTPS://TIMGREIG.CO.UK/2019/10/31/DAY-49",
    "HTTPS://TIMGREIG.CO.UK/2019/11/01/DAY-50",
    "HTTPS://TIMGREIG.CO.UK/2019/11/02/DAY-51",
];

pub static IMAGES: &[&[u8]] = &[
    include_bytes!("../../artwork/shikoku/resized/0.jpg"),
    include_bytes!("../../artwork/shikoku/resized/1.jpg"),
    include_bytes!("../../artwork/shikoku/resized/2.jpg"),
    include_bytes!("../../artwork/shikoku/resized/3.jpg"),
    include_bytes!("../../artwork/shikoku/resized/4.jpg"),
    include_bytes!("../../artwork/shikoku/resized/5.jpg"),
    include_bytes!("../../artwork/shikoku/resized/6.jpg"),
    include_bytes!("../../artwork/shikoku/resized/7.jpg"),
    include_bytes!("../../artwork/shikoku/resized/8.jpg"),
    include_bytes!("../../artwork/shikoku/resized/9.jpg"),
    include_bytes!("../../artwork/shikoku/resized/10.jpg"),
    include_bytes!("../../artwork/shikoku/resized/11.jpg"),
    include_bytes!("../../artwork/shikoku/resized/12.jpg"),
    include_bytes!("../../artwork/shikoku/resized/13.jpg"),
    include_bytes!("../../artwork/shikoku/resized/14.jpg"),
    include_bytes!("../../artwork/shikoku/resized/15.jpg"),
    include_bytes!("../../artwork/shikoku/resized/16.jpg"),
    include_bytes!("../../artwork/shikoku/resized/17.jpg"),
    include_bytes!("../../artwork/shikoku/resized/18.jpg"),
    include_bytes!("../../artwork/shikoku/resized/19.jpg"),
    include_bytes!("../../artwork/shikoku/resized/20.jpg"),
    include_bytes!("../../artwork/shikoku/resized/21.jpg"),
    include_bytes!("../../artwork/shikoku/resized/22.jpg"),
    include_bytes!("../../artwork/shikoku/resized/23.jpg"),
    include_bytes!("../../artwork/shikoku/resized/24.jpg"),
    include_bytes!("../../artwork/shikoku/resized/25.jpg"),
    include_bytes!("../../artwork/shikoku/resized/26.jpg"),
    include_bytes!("../../artwork/shikoku/resized/27.jpg"),
    include_bytes!("../../artwork/shikoku/resized/28.jpg"),
    include_bytes!("../../artwork/shikoku/resized/29.jpg"),
    include_bytes!("../../artwork/shikoku/resized/30.jpg"),
    include_bytes!("../../artwork/shikoku/resized/31.jpg"),
    include_bytes!("../../artwork/shikoku/resized/32.jpg"),
    include_bytes!("../../artwork/shikoku/resized/33.jpg"),
    include_bytes!("../../artwork/shikoku/resized/34.jpg"),
    include_bytes!("../../artwork/shikoku/resized/35.jpg"),
    include_bytes!("../../artwork/shikoku/resized/36.jpg"),
    include_bytes!("../../artwork/shikoku/resized/37.jpg"),
    include_bytes!("../../artwork/shikoku/resized/38.jpg"),
    include_bytes!("../../artwork/shikoku/resized/39.jpg"),
    include_bytes!("../../artwork/shikoku/resized/40.jpg"),
    include_bytes!("../../artwork/shikoku/resized/41.jpg"),
    include_bytes!("../../artwork/shikoku/resized/42.jpg"),
    include_bytes!("../../artwork/shikoku/resized/43.jpg"),
    include_bytes!("../../artwork/shikoku/resized/44.jpg"),
    include_bytes!("../../artwork/shikoku/resized/45.jpg"),
    include_bytes!("../../artwork/shikoku/resized/46.jpg"),
    include_bytes!("../../artwork/shikoku/resized/47.jpg"),
    include_bytes!("../../artwork/shikoku/resized/48.jpg"),
    include_bytes!("../../artwork/shikoku/resized/49.jpg"),
    include_bytes!("../../artwork/shikoku/resized/50.jpg"),
    include_bytes!("../../artwork/shikoku/resized/51.jpg"),
];

pub fn render<D>(display: &mut D, _local: &DateTime, frame: u16, day: u8, animate: bool)
    where D: DrawTarget<Color = Rgb888>
{
    let font = MonoTextStyle::new(&FONT_6X9, Rgb888::WHITE);
    let style = TextStyleBuilder::new()
        .alignment(Alignment::Right)
        .baseline(Baseline::Bottom)
        .build();
    let mut s: String<2> = String::new();

    // Get indices for pixels to draw today.
    let day = (day as usize).min(DAYS.len());
    let mask = if day == 51 { 511 } else { 63 };
    let (route_sidx, temple_sidx) = if animate {
        if day == 0 || day == 51 {
            (0, 0)
        } else {
            (DAYS[day - 1] as usize, N_TEMPLES[day - 1] as usize)
        }
    } else {
        (DAYS[day] as usize, N_TEMPLES[day] as usize)
    };
    let route_eidx = u16::min(DAYS[day], (route_sidx as u16) + (frame & mask)) as usize;
    let temple_eidx = N_TEMPLES[day] as usize;

    // Render route up til the start of today.
    for (x, y) in ROUTE[..route_sidx].iter() {
        Pixel(Point::new(*x as i32, *y as i32), Rgb888::WHITE).draw(display).ok();
    }

    // Render today's section of the route.
    for (x, y) in ROUTE[route_sidx..route_eidx].iter() {
        Pixel(Point::new(*x as i32, *y as i32), Rgb888::WHITE).draw(display).ok();
    }

    // Render already visited temples.
    for (x, y) in TEMPLES[..temple_sidx].iter() {
        Pixel(Point::new(*x as i32, *y as i32), TEMPLE_COLOR).draw(display).ok();
    }

    // Render today's temples once they've been visited by today's route section.
    for (x, y) in TEMPLES[temple_sidx..temple_eidx].iter() {
        if ROUTE[route_sidx..route_eidx].contains(&(*x, *y)) {
            Pixel(Point::new(*x as i32, *y as i32), TEMPLE_COLOR).draw(display).ok();
        }
    }

    // Walk day
    write!(&mut s, "{:2}", day).ok();
    Text::with_text_style(&s, Point::new(37, 64), font, style).draw(display).ok();
    s.clear();

    // Number of temples
    write!(&mut s, "{:2}", N_TEMPLES[day as usize]).ok();
    Text::with_text_style(&s, Point::new(55, 64), font, style).draw(display).ok();
}
