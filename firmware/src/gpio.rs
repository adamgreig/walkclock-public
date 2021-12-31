#![allow(dead_code)]

use stm32ral::{gpio, read_reg, write_reg, modify_reg};

type Gpio = gpio::Instance;

/// [`InputPin`] for each switch.
#[allow(clippy::manual_non_exhaustive)]
pub struct Switches {
    pub back: InputPin,
    pub qr: InputPin,
    pub display: InputPin,
    pub enter: InputPin,
    pub left: InputPin,
    pub right: InputPin,
    _private: (),
}

/// HUB75E interface.
///
/// Provides methods for efficiently setting the address lines, and accessing the ODR register
/// address for the data, clock, and latch lines.
pub struct Hub {
    addr_bsrr: u32,
    addr_offset: usize,
    data_odr: u32,
}

impl Hub {
    /// Set the address lines A-E to the provided 5-bit value.
    pub fn set_addr(&self, addr: u8) {
        // NOTE(unsafe): Write-only access to atomic bit-set register.
        // NOTE(unsafe): No other code accesses this register.
        unsafe {
            core::ptr::write_volatile(
                self.addr_bsrr as *mut u32,
                // Reset all address bits.
                (0b11111 << (self.addr_offset + 16)) |
                // Set relevant bits, taking priority over reset.
                ((addr as u32) << self.addr_offset)
            );
        }
    }

    /// Get the address of the data register to DMA into for
    /// R1, G1, B1, R2, G2, B2, LAT, OE signals.
    pub fn odr(&self) -> u32 {
        self.data_odr
    }
}

/// Pins container.
///
/// Contains the results of setting up the GPIOs,
/// including access to switches, the HUB75E interface, and the GPIOs.
#[allow(clippy::manual_non_exhaustive)]
pub struct Pins {
    pub switches: Switches,
    pub hub: Hub,
    pub gps_reset: OutputPin,
    pub led: OutputPin,
    pub lcd_wr_rs: OutputPin,
    _private: (),
}

pub fn setup(gpioa: Gpio, gpiob: Gpio, gpioc: Gpio, _gpiod: Gpio, gpioe: Gpio) -> Pins {
    // GPIOA
    // PA0: Input pulled up, SW_BACK
    // PA1: Input pulled up, SW_ENTER
    // PA2: Input pulled up, SW_QR
    // PA3: Input pulled up, SW_LEFT
    // PA4: Input pulled up, SW_DISP
    // PA5: Input pulled up, SW_RIGHT
    // PA6-7: Unused
    // PA8: AF0, MCO1 for LSE calibration
    // PA9-12: Unused
    // PA13: AF0 SWDIO pulled up
    // PA14: AF0 SWCLK pulled down
    // PA15: Unused.
    write_reg!(gpio, gpioa, ODR, 0);
    write_reg!(gpio, gpioa, MODER, MODER0: Input, MODER1: Input, MODER2: Input,
                                   MODER3: Input, MODER4: Input, MODER5: Input,
                                   MODER8: Alternate, MODER13: Alternate, MODER14: Alternate);
    write_reg!(gpio, gpioa, PUPDR, PUPDR0: PullUp, PUPDR1: PullUp, PUPDR2: PullUp,
                                   PUPDR3: PullUp, PUPDR4: PullUp, PUPDR5: PullUp,
                                   PUPDR13: PullUp, PUPDR14: PullDown);
    write_reg!(gpio, gpioa, AFRH, AFR8: 0, AFR13: 0, AFR14: 0);

    // GPIOB
    // PB0-PB6: Unused
    // PB7: Output, GPS_RESET
    // PB8-9: Unused
    // PB10: AF1, TIM2_CH3, HUB_CLK
    // PB11-14: Unused
    // PB15: AF0, RTC_REFIN
    write_reg!(gpio, gpiob, ODR, 0);
    write_reg!(gpio, gpiob, MODER, MODER7: Output, MODER10: Alternate, MODER15: Alternate);
    write_reg!(gpio, gpiob, OSPEEDR, OSPEEDR10: MediumSpeed);
    write_reg!(gpio, gpiob, AFRH, AFR10: 1, AFR15: 0);

    // GPIOC
    // PC0: Output, HUB_R1
    // PC1: Output, HUB_G1
    // PC2: Output, HUB_B1
    // PC3: Output, HUB_R2
    // PC4: Output, HUB_G2
    // PC5: Output, HUB_B2
    // PC6: Output, HUB_LAT
    // PC7: AF2, TIM3_CH2, HUB_OE
    // PC8-PC12: Unused
    // PC13: Input, KEY / RTC_CALIB
    // PC14-PC15: Unused
    // HUB pins need to be medium speed <=60MHz.
    write_reg!(gpio, gpioc, ODR, 0);
    write_reg!(gpio, gpioc, MODER, MODER0: Output, MODER1: Output, MODER2: Output, MODER3: Output,
                                   MODER4: Output, MODER5: Output, MODER6: Output,
                                   MODER7: Alternate, MODER13: Input);
    write_reg!(gpio, gpioc, OSPEEDR, OSPEEDR0: MediumSpeed, OSPEEDR1: MediumSpeed,
                                     OSPEEDR2: MediumSpeed, OSPEEDR3: MediumSpeed,
                                     OSPEEDR4: MediumSpeed, OSPEEDR5: MediumSpeed,
                                     OSPEEDR6: MediumSpeed, OSPEEDR7: MediumSpeed);
    write_reg!(gpio, gpioc, AFRL, AFR7: 2);

    // GPIOD
    // PD0-15: Unused

    // GPIOE
    // PE0: AF8, UART8_RX, GPS_TX
    // PE1: AF8, UART8_TX, GPS_RX
    // PE2: Unused
    // PE3: Output, LED
    // PE4: Unused
    // PE5: Output, HUB_A
    // PE6: Output, HUB_B
    // PE7: Output, HUB_C
    // PE8: Output, HUB_D
    // PE9: Output, HUB_E
    // PE10: AF1, TIM1_CH2N, LCD_LED
    // PE11: AF5, SPI4_NSS, LCD_CS
    // PE12: AF5, SPI4_SCK, LCD_SCL
    // PE13: Output, LCD_WR_RS
    // PE14: AF5, SPI4_MOSI, LCD_SDA
    // PE15: Unused
    // HUB pins and SPI pins need to be medium speed <=60MHz.
    write_reg!(gpio, gpioe, ODR, 0);
    write_reg!(gpio, gpioe, MODER, MODER0: Alternate, MODER1: Alternate, MODER3: Output,
                                   MODER5: Output, MODER6: Output, MODER7: Output, MODER8: Output,
                                   MODER9: Output, MODER10: Alternate, MODER11: Alternate,
                                   MODER12: Alternate, MODER13: Output, MODER14: Alternate);
    write_reg!(gpio, gpioe, OSPEEDR, OSPEEDR0: LowSpeed, OSPEEDR1: LowSpeed, OSPEEDR3: LowSpeed,
                                     OSPEEDR5: MediumSpeed, OSPEEDR6: MediumSpeed,
                                     OSPEEDR7: MediumSpeed, OSPEEDR8: MediumSpeed,
                                     OSPEEDR9: MediumSpeed, OSPEEDR10: LowSpeed,
                                     OSPEEDR11: MediumSpeed, OSPEEDR12: MediumSpeed,
                                     OSPEEDR13: LowSpeed, OSPEEDR14: MediumSpeed);
    write_reg!(gpio, gpioe, PUPDR, PUPDR0: PullUp);
    write_reg!(gpio, gpioe, AFRL, AFR0: 8, AFR1: 8);
    write_reg!(gpio, gpioe, AFRH, AFR10: 1, AFR11: 5, AFR12: 5, AFR14: 5);

    let switches = Switches {
        back:       InputPin::new(&gpioa, 0),
        qr:         InputPin::new(&gpioa, 2),
        display:    InputPin::new(&gpioa, 4),
        enter:      InputPin::new(&gpioa, 1),
        left:       InputPin::new(&gpioa, 3),
        right:      InputPin::new(&gpioa, 5),
        _private: (),
    };

    let hub = Hub {
        addr_bsrr: &gpioe.BSRR as *const _ as u32,
        addr_offset: 5,
        data_odr: &gpioc.ODR as *const _ as u32,
    };

    Pins {
        switches,
        hub,
        gps_reset:  OutputPin::new(&gpiob, 7),
        led:        OutputPin::new(&gpioe, 3),
        lcd_wr_rs:  OutputPin::new(&gpioe, 13),
        _private: (),
    }
}

/// Pin for runtime control of outputs.
pub struct OutputPin {
    bsrr: u32,
    pin: u32,
}

impl OutputPin {
    /// Construct a new OutputPin from a given GPIO instance and pin number.
    fn new(port: &gpio::Instance, pin: u32) -> OutputPin {
        OutputPin {
            bsrr: &port.BSRR as *const _ as u32, pin
        }
    }

    /// Set pin low if `level` is 0, otherwise set it high.
    pub fn set(&self, level: u32) {
        // NOTE(unsafe): Write into a write-only atomic register.
        unsafe {
            if level == 0 {
                core::ptr::write_volatile(self.bsrr as *mut u32, 1 << (self.pin + 16));
            } else {
                core::ptr::write_volatile(self.bsrr as *mut u32, 1 << self.pin);
            }
        }
    }

    /// Set pin high.
    pub fn set_high(&self) {
        self.set(1);
    }

    /// Set pin low.
    pub fn set_low(&self) {
        self.set(0);
    }
}

/// Pin for runtime reading of inputs.
pub struct InputPin {
    idr: u32,
    pin: u32,
}

impl InputPin {
    /// Construct a new InputPin from a given GPIO instance and pin number.
    fn new(port: &gpio::Instance, pin: u32) -> Self {
        InputPin {
            idr: &port.IDR as *const _ as u32, pin
        }
    }

    /// Reads current pin state.
    ///
    /// Returns true for a high level and false for a low level.
    pub fn get(&self) -> bool {
        // NOTE(unsafe): Read from a read-only register.
        unsafe {
            (core::ptr::read_volatile(self.idr as *const u32) >> self.pin) & 1 == 1
        }
    }
}

/// Force on the onboard LED from any context.
pub fn led_on() {
    // NOTE(unsafe): Atomic write-only register.
    unsafe {
        write_reg!(gpio, GPIOE, BSRR, BS3: 1);
    }
}

/// Force off the onboard LED from any context.
pub fn led_off() {
    // NOTE(unsafe): Atomic write-only register.
    unsafe {
        write_reg!(gpio, GPIOE, BSRR, BR3: 1);
    }
}

/// Force PB15 to AF0, used for RTC REF_IN from the GPS.
pub fn pb15_rtc() {
    // NOTE(unsafe): No other code accesses this register outside of the initialisation
    // NOTE(unsafe): routine above and `pb15_tim()` below, which changes the same field.
    unsafe {
        if read_reg!(gpio, GPIOB, AFRH, AFR15 != 0) {
            modify_reg!(gpio, GPIOB, AFRH, AFR15: 0);
        }
    }
}

/// Force PB15 to AF2, used for TIM12 CH2 input for calibration measurement.
pub fn pb15_tim() {
    // NOTE(unsafe): No other code accesses this register outside of the initialisation
    // NOTE(unsafe): routine above and `pb15_rtc()` above, which changes the same field.
    unsafe {
        if read_reg!(gpio, GPIOB, AFRH, AFR15 != 2) {
            modify_reg!(gpio, GPIOB, AFRH, AFR15: 2);
        }
    }
}
