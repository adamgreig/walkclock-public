#![no_std]
#![no_main]

use panic_rtt_target as _;
mod dma;
mod dma2d;
mod framebuf;
mod gpio;
mod hub75e;
mod lcd;
mod rcc;
mod rtc;
mod spi;
mod switches;
mod tim;
mod uart;
mod ublox;

mod jpeg;

pub type LineBuf = [u8; 65];

#[rtic::app(device=stm32ral::stm32h7::stm32h743v, dispatchers=[WKUP])]
mod app {
    use crate::{
        dma, dma2d, gpio, jpeg, rcc, rtc, spi, tim, uart,
        lcd::Lcd, ublox::{UBlox, PVTError}, switches::Switches, hub75e::Hub75E,
        LineBuf, framebuf::{FrameBuf, MainFrameBuf, SubFrameBuf},
    };
    use rtt_target::{rtt_init_print, rprintln, rprint};
    use walkclock::Clock;

    /// Double-buffered RGB888 frame buffers for main HUB75E display.
    static mut MFBUFS: [MainFrameBuf; 2] = [FrameBuf([[[0u8; 3]; 64]; 64]); 2];

    /// Output buffers for HUB75E driver, which is read by DMA1 so goes in SRAM1.
    ///
    /// NOTE: Despite being set to 0, SRAM1 is not initialised by the runtime,
    /// so this buffer starts life uninitialised. However, we only ever write
    /// to it from Rust and it's only read by DMA, so we sort of avoid UB.
    #[link_section=".sram1.lbufs"]
    static mut LBUFS: [LineBuf; 2] = [[0u8; 65]; 2];

    /// Single-buffered RGB888 frame buffer for LCD display.
    ///
    /// NOTE: Stored in SRAM2 to enable direct access by DMA,
    ///       while not contending with SRAM1 for the main display.
    #[link_section=".sram2.sfbuf"]
    static mut SFBUF: SubFrameBuf = FrameBuf([[[0u8; 3]; 160]; 80]);

    /// Reception buffer for UART.
    /// NOTE: Stored in SRAM2 to enable direct access by DMA.
    #[link_section=".sram2.uartbuf"]
    static mut UARTBUF: [u8; 100] = [0u8; 100];

    /// TIM15 capture buffer.
    /// Captures 2049 LSE edge timestamps to reconstruct 2048 LSE period measurements,
    /// or half a second worth of LSE ticks at 32768Hz LSE with timer input prescaler 8.
    /// NOTE: Stored in SRAM2 to enable direct access by DMA.
    #[link_section=".sram2.tim15buf"]
    static mut TIM15BUF: [u16; 2049] = [0u16; 2049];

    /// JPEG decode buffer.
    /// Stores decoded MCUs from the most recently processed JPEG.
    /// NOTE: Stored in AXISRAM as it's spacious and can be accessed by CPU and DMA2D.
    #[link_section=".axisram.jpegdbuf"]
    static mut JPEGDBUF: [u32; 3072] = [0u32; 3072];

    /// JPEG framebuffer.
    /// Stores converted JPEG RGB888 pixels ready for copying into active framebuffer.
    /// NOTE: Stored in AXISRAM as it's spacious and can be accessed by DMA2D.
    #[link_section=".axisram.jpegfbuf"]
    static mut JPEGFBUF: MainFrameBuf = FrameBuf([[[0u8; 3]; 64]; 64]);

    #[shared]
    struct Shared {
        hub: Hub75E,
        lcd: Lcd,
        ublox: UBlox,
        cal: rtc::Calibrator,
    }

    #[local]
    struct Local {
        rtc: rtc::RTC,
        tick_tim: tim::Tim,
        tim12: tim::Tim,
        tim15_dma: dma::DMAStream,
        jpeg: jpeg::Jpeg,
        dma2d: dma2d::DMA2D,
        clock: Clock,
        switches: Switches,
    }

    #[init]
    fn init(mut cx: init::Context) -> (Shared, Local, init::Monotonics) {
        rtt_init_print!();
        rprintln!("WalkClock initialising...");

        // Set up clocks, including PWR voltage scaling and flash wait states.
        rprint!("  RCC...      ");
        let clocks = rcc::setup(cx.device.RCC, cx.device.PWR, cx.device.FLASH);
        rprintln!("OK");

        rprint!("  ICACHE...   ");
        cx.core.SCB.enable_icache();
        rprintln!("OK");

        /*
        rprintln!("Testing JPEG decoder...");
        crate::jpeg::test(cx.device.JPEG);
        rprintln!("Done, continuing...");
        */
        rprint!("  JPEG...     ");
        let jpeg = jpeg::Jpeg::new(cx.device.JPEG);
        rprintln!("OK");

        rprint!("  GPIO...     ");
        let pins = gpio::setup(cx.device.GPIOA, cx.device.GPIOB, cx.device.GPIOC,
                               cx.device.GPIOD, cx.device.GPIOE);
        rprintln!("OK");

        rprint!("  Switches... ");
        // Repeat after the first second and then four times per second thereafter.
        let switches = Switches::new(pins.switches, 20, 5);
        rprintln!("OK");

        rprint!("  RTC...      ");
        let rtc = rtc::RTC::new(cx.device.RTC);
        rprintln!("OK");

        rprint!("  TIM...      ");
        // TIM1 is used to generate the LCD backlight PWM at 250kHz and 1/3 duty cycle.
        // TIM2 is used to generate the HUB75E pixel clock at 15MHz.
        // TIM3 is used to generate the HUB75E OE pulses.
        // TIM4 is used to generate the 20Hz display update ticks, prescaled to 10kHz.
        // TIM15 is used to measure LSE, sending timestamps to a buffer via DMA.
        // If need be, TIM2 can be upped to 18.75MHz for a little more brightness.
        let tim1 = tim::Tim::from_tim1(cx.device.TIM1);
        tim1.setup_lcd_pwm(clocks.tim_ck / 250_000);
        tim1.set_lcd_duty((clocks.tim_ck / 250_000) / 3);
        let tim2 = tim::Tim::from_tim2(cx.device.TIM2);
        tim2.setup_hub_clk(clocks.tim_ck / 15_000_000);
        let tim3 = tim::Tim::from_tim3(cx.device.TIM3);
        tim3.setup_hub_oe();
        let tim4 = tim::Tim::from_tim4(cx.device.TIM4);
        tim4.setup_tick(clocks.tim_ck / 10_000, 500);
        let tim12 = tim::Tim::from_tim12(cx.device.TIM12);
        tim12.setup_psc50_ti2();
        let tim15 = tim::Tim::from_tim15(cx.device.TIM15);
        tim15.setup_tim15_lse();
        rprintln!("OK");

        rprint!("  SPI...      ");
        let spi4 = spi::Spi::new(cx.device.SPI4);
        spi4.setup_lcd(&clocks);
        rprintln!("OK");

        rprint!("  UART...     ");
        let uart8 = uart::Uart::new(cx.device.UART8);
        uart8.setup_ublox(&clocks);
        rprintln!("OK");

        rprint!("  DMA...      ");
        // ID 20  TIM2_CH3  => DMAMUX 0  => DMA1 stream 0
        // ID 81  UART8_RX  => DMAMUX 8  => DMA2 stream 0
        // ID 84  SPI4_TX   => DMAMUX 9  => DMA2 stream 1
        // ID 105 TIM15_CH1 => DMAMUX 10 => DMA2 stream 2
        let dmamux = dma::DMAMux1::new(cx.device.DMAMUX1);
        dmamux.set(0, 20);
        dmamux.set(8, 81);
        dmamux.set(9, 84);
        dmamux.set(10, 105);
        let dma1 = dma::DMA::new(cx.device.DMA1);
        dma1.s0.setup_tx(pins.hub.odr());
        let dma2 = dma::DMA::new(cx.device.DMA2);
        dma2.s0.setup_rx(uart8.rdr());
        dma2.s0.set_trbuff();
        dma2.s1.setup_tx(spi4.txdr());
        dma2.s2.setup_u16_rx(tim15.ccr1());
        let dma2d = dma2d::DMA2D::new(cx.device.DMA2D);
        rprintln!("OK");

        rprint!("  uBlox...    ");
        // NOTE(unsafe): DMA writes to this buffer which must live in SRAM2 and be static,
        // NOTE(unsafe): but we are careful to only read when DMA is not active.
        let buf = unsafe { &mut UARTBUF };
        let ublox = UBlox::new(uart8, pins.gps_reset, dma2.s0, buf);
        ublox.setup();
        rprintln!("OK");

        rprint!("  Clock...    ");
        let mut clock = Clock::new();
        let mut settings = [0u32; 32];
        rtc.read_backup(&mut settings[..]);
        clock.deserialise(&settings[..]);
        rprintln!("OK");

        rprint!("  LCD...      ");
        let lcd = Lcd::new(spi4, pins.lcd_wr_rs, dma2.s1);
        lcd.start();
        rprintln!("OK");

        rprint!("  HUB...      ");
        // NOTE(unsafe): We manage frame buffer swapping manually, with the shared reference
        // NOTE(unsafe): created here being read by the HUB75E driver, and later swapped
        // NOTE(unsafe): out for a freshly drawn buffer.
        let mfbuf = unsafe { &MFBUFS[1] };
        // NOTE(unsafe): The line buffers are only accessed through this single mutable reference.
        let lbufs = unsafe { &mut LBUFS };
        // We use bcm_base=18, giving 18/150MHz = 120ns as the smallest OE pulse.
        let mut hub = Hub75E::new(pins.hub, tim2, tim3, dma1.s0, mfbuf, lbufs, 18);
        hub.start();
        rprintln!("OK");

        rprintln!("Initialisation complete.");

        // Start capturing LSE measurements.
        // NOTE(unsafe): Written to by DMA, then read in DMA interrupt while DMA is stopped.
        dma2.s2.start_u16_rx(unsafe { &mut TIM15BUF[..]});

        // Start timer for 20Hz main render loop ticks.
        tim4.start();

        (
            Shared {
                hub,
                lcd,
                ublox,
                cal: rtc::Calibrator::new(),
            },

            Local {
                rtc,
                tick_tim: tim4,
                tim12,
                tim15_dma: dma2.s2,
                jpeg,
                dma2d,
                clock,
                switches,
            },

            init::Monotonics {}
        )
    }

    /// Empty idle handler prevents low-power sleep mode
    /// between interrupts, which takes long enough to resume
    /// from that it can interfere with HUB75E timing.
    #[idle]
    fn idle(_: idle::Context) -> ! {
        loop {}
    }

    /// HUB75 DMA write completion interrupt.
    #[task(binds=DMA_STR0, priority=5, shared=[hub])]
    fn dma_hub75(mut cx: dma_hub75::Context) {
        cx.shared.hub.lock(|hub| hub.dma_isr());
    }

    /// HUB75 OE pulse completion interrupt.
    #[task(binds=TIM3, priority=5, shared=[hub])]
    fn tim_oe(mut cx: tim_oe::Context) {
        cx.shared.hub.lock(|hub| hub.tim_oe_isr());
    }

    /// LCD DMA write completion interrupt.
    #[task(binds=DMA2_STR1, priority=4, shared=[lcd])]
    fn dma_lcd(mut cx: dma_lcd::Context) {
        cx.shared.lcd.lock(|lcd| lcd.dma_isr());
    }

    /// UART DMA interrupt.
    #[task(binds=DMA2_STR0, priority=3, shared=[ublox])]
    fn dma_ublox(mut cx: dma_ublox::Context) {
        cx.shared.ublox.lock(|ublox| ublox.dma_isr());
    }

    /// UART IDLE interrupt.
    #[task(binds=UART8, priority=3, shared=[ublox])]
    fn uart_ublox(mut cx: uart_ublox::Context) {
        cx.shared.ublox.lock(|ublox| ublox.uart_isr());
    }

    /// TIM12 interrupt.
    /// Configured for CC2 capture, which measures the period of the GPS 50Hz timepulse.
    /// Sums the difference of the last 50 CC2 captures to effectively measure HSE frequency.
    #[task(
        binds=TIM8_BRK_TIM12,
        local=[tim12, n: u32 = 0, s: u32 = 0, p: u16 = 0],
        shared=[cal],
        priority=1
    )]
    fn tim12(mut cx: tim12::Context) {
        cx.local.tim12.clear_uif();
        let cc2 = cx.local.tim12.cc2() as u16;
        *cx.local.n += 1;
        *cx.local.s += (cc2 - *cx.local.p) as u32;
        *cx.local.p = cc2;
        if *cx.local.n == 50 {
            cx.shared.cal.lock(|cal| cal.gps_reading(*cx.local.s));
            *cx.local.n = 0;
            *cx.local.s = 0;
        }
    }

    /// TIM15 DMA interrupt.
    /// Computes the number of APB ticks for 16384 ticks of LSE.
    #[task(binds=DMA2_STR2, priority=1, local=[tim15_dma], shared=[cal])]
    fn tim15_dma(mut cx: tim15_dma::Context) {
        // Compute APB ticks between first and last LSE tick by summing the difference
        // between each timestamped tick.
        // NOTE(unsafe): DMA is disabled when this interrupt is entered until we restart it.
        let buf = unsafe { &TIM15BUF[..] };
        let sum: u32 = buf.windows(2).map(|w| (w[1] - w[0]) as u32).sum();
        cx.shared.cal.lock(|cal| cal.lse_reading(sum));

        // Restart DMA processing.
        // NOTE(unsafe): Written to by DMA, then read in DMA interrupt while DMA is stopped.
        cx.local.tim15_dma.start_u16_rx(unsafe { &mut TIM15BUF[..]});
    }

    /// Main loop 20Hz timer tick.
    ///
    /// Processes new GPS messages, handles updates from the clock application,
    /// then renders both displays.
    #[task(
        binds=TIM4,
        priority=2,
        local=[
            rtc, clock, tick_tim, switches, jpeg, dma2d,
            fbuf_idx: usize = 0, nolock_time: u32 = 0, prev_jpeg: u32 = 0,
        ],
        shared=[hub, lcd, ublox, cal],
    )]
    fn tim_tick(mut cx: tim_tick::Context) {
        cx.local.tick_tim.clear_uif();

        // Process button inputs.
        cx.local.switches.update();
        if cx.local.switches.back() {
            cx.local.clock.key_back();
        }
        if cx.local.switches.qr() {
            cx.local.clock.key_qr();
        }
        if cx.local.switches.display() {
            cx.local.clock.key_display();
        }
        if cx.local.switches.enter() {
            cx.local.clock.key_enter();
        }
        if cx.local.switches.left() {
            cx.local.clock.key_left();
        }
        if cx.local.switches.right() {
            cx.local.clock.key_right();
        }

        if cx.local.clock.use_gps_time() {
            // Process any newly received GNSS times.
            match cx.shared.ublox.lock(|ublox| ublox.pvt()) {
                Ok(pvt) => {
                    if pvt.fix {
                        *cx.local.nolock_time = 0;
                        if pvt.valid_date && pvt.valid_time && pvt.fully_resolved {
                            cx.local.clock.set_gps_lock_valid(pvt.num_sv);
                            cx.local.rtc.new_pvt(&pvt);
                        } else {
                            cx.local.clock.set_gps_lock_invalid(pvt.num_sv);
                        }
                    } else {
                        *cx.local.nolock_time += 1;
                        cx.local.clock.set_gps_no_lock(*cx.local.nolock_time);
                    }
                },

                // Allow 30 NoPVTs in a row before declaring an error due to missing data.
                // At 20Hz render loop, we only expect to see one PVT every 20 cycles anyway.
                Err(PVTError::NoPVT(n)) => if n > 30 {
                    cx.local.clock.set_gps_error();
                    if n % 32 == 0 {
                        // Rate-limit timeout error printing just to avoid spamming rtt console.
                        rprintln!("GPS timeout: {:?}", n);
                    }
                },

                // Any other error is an immediate failure we can report.
                Err(e) => {
                    cx.local.clock.set_gps_error();
                    rprintln!("GPS error: {:?}", e);
                }
            }
        } else {
            // Indicate that GPS is not being used on the status screen.
            cx.local.clock.set_gps_unused();

            // Check if the user has changed the clock time via the menu, and update RTC if so.
            if cx.local.clock.time_changed() {
                let (year, month, day, hour, minute, second) = cx.local.clock.menu_time();
                let year = (year - 2000) as u8;
                let dt = rtc::DateTime { year, month, day, hour, minute, second };
                cx.local.rtc.set(&dt);
            }
        }

        // Get the current brightness from the application and convert
        // to number of BCM phases to skip.
        let bcm_skip = 10 - cx.local.clock.brightness();

        // Update the application with the latest date and time from the RTC.
        let time = cx.local.rtc.read();
        cx.local.clock.set_time(
            time.year as u16 + 2000, time.month, time.day, time.hour, time.minute, time.second);

        // At the middle of each hour, process potential RTC calibration.
        if time.minute == 30 {
            if time.second == 0 {
                // At 0 seconds, clear any old saved data and enable capturing new data.
                cx.shared.cal.lock(|cal| cal.clear());
                gpio::pb15_tim();
            } else if time.second == 6 {
                // At 6 seconds, apply a calibration if valid, and set GPIO back to RTC ref.
                if let Some((calp, calm)) = cx.shared.cal.lock(|cal| cal.cal()) {
                    rprintln!("Setting RTC calibration to CALP={} CALM={}", calp, calm);
                    cx.local.rtc.set_calibration(calp, calm);
                }
                gpio::pb15_rtc();
            }
        }

        // If the application state has changed, save it to backup registers.
        if cx.local.clock.needs_saving() {
            let mut settings = [0u32; 32];
            cx.local.clock.serialise(&mut settings[..]);
            cx.local.rtc.write_backup(&settings[..]);
        }

        // NOTE(unsafe): Get the frame buffer not currently used by the HUB75E driver,
        // NOTE(unsafe): we'll write to it and then swap it into the driver.
        let mfbuf = unsafe { &mut MFBUFS[*cx.local.fbuf_idx] };

        // Check if we need to render a JPEG first.
        if let Some(jpeg) = cx.local.clock.prerender_jpeg() {
            // NOTE(unsafe): JPEGFBUF may be written to by DMA2D during this lifetime,
            // NOTE(unsafe): and then is copied into MFBUF once DMA2D is complete.
            let jpegfbuf = unsafe { &mut JPEGFBUF };

            if jpeg.as_ptr() as u32 != *cx.local.prev_jpeg {
                // NOTE(unsafe): JPEGDBUF will only be written to by the JPEG peripheral
                // NOTE(unsafe): and then read by DMA2D, all during the lifetime of this ref.
                let jpegdbuf = unsafe { &mut JPEGDBUF };

                // Decode provided JPEG.
                if let Err(e) = cx.local.jpeg.decode(jpeg, jpegdbuf) {
                    rprintln!("Error decoding JPEG: {:?}", e);
                    mfbuf.clear_black();
                }

                // Convert (possibly already decoded) JPEG data into RGB888 in the framebuffer.
                if let Err(e) = cx.local.dma2d.convert_jpeg(jpegdbuf, jpegfbuf) {
                    rprintln!("Error converting JPEG: {:?}", e);
                    mfbuf.clear_black();
                }

                // Remember the last JPEG we decoded to save reprocessing it next time.
                *cx.local.prev_jpeg = jpeg.as_ptr() as u32;
            }

            // Copy the processed JPEG image into the main framebuffer in DTCM.
            mfbuf.0.copy_from_slice(&jpegfbuf.0);

        } else {

            // If not drawing a JPEG, clear the screen to black instead.
            mfbuf.clear_black();
        }

        // Render the main display.
        cx.local.clock.render_main(mfbuf);
        cx.shared.hub.lock(|hub| {
            hub.set_bcm_skip(bcm_skip);
            hub.set_fbuf(mfbuf);
        });

        // Use next framebuffer next time.
        *cx.local.fbuf_idx ^= 1;

        // NOTE(unsafe): While we cannot verify it statically, our realtime deadline for
        // NOTE(unsafe): memory safety is that the DMA transfer of the previous render
        // NOTE(unsafe): completes before this render operation. At 9.375MHz SPI clock,
        // NOTE(unsafe): it takes 32.768ms to write one frame, and we render every 50ms.
        let sfbuf = unsafe { &mut SFBUF };

        // Render the sub display and trigger the DMA write to the SPI LCD.
        sfbuf.clear_black();
        cx.local.clock.render_sub(sfbuf);
        cx.shared.lcd.lock(|lcd| lcd.write_fbuf(sfbuf));
    }
}
