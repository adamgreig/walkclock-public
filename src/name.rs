/// All names used throughout the library which must occasionally be displayed as strings.
///
/// The strings are effectively interned; each only appears in memory once but is represented
/// by an enum, allowing efficient copying, comparisons, and storage.
#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(u8)]
pub enum Name {
    Unused,
    DateTime,
    GPSTime,
    Year,
    Month,
    Day,
    Hour,
    Minute,
    Second,
    AutomaticDST,
    UTCOffset,
    Map,
    Route,
    RouteDay,
    AnimateRoute,
    HourlyImages,
    Display,
    Brightness,
    DimAtNight,
    DimBrightness,
    DimStartHour,
    DimEndHour,
    CaminoFrances,
    HolyIsland,
    Scotland,
    Shikoku,
    ViaFrancigena,
    ViaPodiensis,
    NoMap,
}

impl From<&Name> for &'static str {
    fn from(name: &Name) -> &'static str {
        match name {
            Name::Unused        => "",              //
            Name::DateTime      => "Date/Time",     //
            Name::GPSTime       => "GPS time",      //
            Name::Year          => "Year (UTC)",    //
            Name::Month         => "Month (UTC)",   //
            Name::Day           => "Day (UTC)",     //
            Name::Hour          => "Hour (UTC)",    //
            Name::Minute        => "Minute (UTC)",  //
            Name::Second        => "Second (UTC)",  //
            Name::AutomaticDST  => "Automatic DST", //
            Name::UTCOffset     => "UTC offset",    //
            Name::Map           => "Map",           //
            Name::Route         => "Route",         //
            Name::RouteDay      => "Route day",     //
            Name::AnimateRoute  => "Animate route", //
            Name::HourlyImages  => "Hourly images", //
            Name::Display       => "Display",       //
            Name::Brightness    => "Brightness",    //
            Name::DimAtNight    => "Dim at night",  //
            Name::DimBrightness => "Dim brightness",//
            Name::DimStartHour  => "Dim start hour",//
            Name::DimEndHour    => "Dim end hour",  //
            Name::CaminoFrances => "Camino Frances",//
            Name::HolyIsland    => "Holy Island",   //
            Name::Scotland      => "Scotland",      //
            Name::ViaFrancigena => "Via Francigena",//
            Name::ViaPodiensis  => "Via Podiensis", //
            Name::Shikoku       => "Shikoku",       //
            Name::NoMap         => "None",          //
        }
    }
}

impl core::fmt::Display for Name {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.into())
    }
}

impl Name {
    pub fn into_str(&self) -> &'static str {
        self.into()
    }
}
