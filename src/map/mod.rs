use embedded_graphics::{pixelcolor::Rgb888, prelude::*};
use crate::{Name, clock::DateTime};

pub mod shikoku;

#[derive(Copy, Clone, Debug)]
pub enum Map {
    Shikoku,
}

pub const MAP_NAMES: &[Name] = &[Name::NoMap, Name::Shikoku];

impl Map {
    pub fn background(&self) -> Option<&'static [u8]> {
        match self {
            Self::Shikoku       => Some(shikoku::IMAGE),
        }
    }

    pub fn render<D>(&self, display: &mut D, local: &DateTime, frame: u16, day: u8, animate: bool)
        where D: DrawTarget<Color = Rgb888>
    {
        match self {
            Self::Shikoku   => shikoku::render(display, local, frame, day, animate),
        }
    }

    pub fn name(&self) -> Name {
        match self {
            Self::Shikoku       => Name::Shikoku,
        }
    }

    pub fn days(&self) -> usize {
        match self {
            Self::Shikoku       => shikoku::DAYS.len(),
        }
    }

    pub fn url(&self, day: u8) -> Option<&'static str> {
        let day = day as usize;
        match self {
            Self::Shikoku if day < shikoku::URLS.len()      => Some(shikoku::URLS[day]),
            _                                               => None,
        }
    }

    pub fn image(&self, day: u8) -> Option<&'static [u8]> {
        let day = day as usize;
        match self {
            Self::Shikoku if day < shikoku::IMAGES.len()    => Some(shikoku::IMAGES[day]),
            _                                               => None,
        }
    }
}

impl TryFrom<Name> for Map {
    type Error = ();
    fn try_from(name: Name) -> Result<Map, ()> {
        match name {
            Name::Shikoku           => Ok(Map::Shikoku),
            _                       => Err(()),
        }
    }
}
