use stm32ral::{modify_reg, write_reg, read_reg};
use stm32ral::{
    tim1, tim2, tim3, tim4, tim5, tim6, tim7, tim8, tim12, tim13, tim14, tim15, tim16, tim17,
};

/// Generic timer driver.
///
/// This driver does not type-check the provided timer peripheral, and so
/// if used incorrectly may try to enable an output on a timer without one.
pub struct Tim {
    tim: tim1::Instance,
}

macro_rules! impl_tim {
    ($type:ident, $fn:ident) => {
        pub fn $fn(tim: $type::Instance) -> Self {
            // NOTE(unsafe): We'll only transmute various types of timer instance to common TIM1.
            Tim { tim: unsafe { core::mem::transmute(tim) } }
        }
    }
}

impl Tim {
    impl_tim!(tim1, from_tim1);
    impl_tim!(tim2, from_tim2);
    impl_tim!(tim3, from_tim3);
    impl_tim!(tim4, from_tim4);
    impl_tim!(tim5, from_tim5);
    impl_tim!(tim6, from_tim6);
    impl_tim!(tim7, from_tim7);
    impl_tim!(tim8, from_tim8);
    impl_tim!(tim12, from_tim12);
    impl_tim!(tim13, from_tim13);
    impl_tim!(tim14, from_tim14);
    impl_tim!(tim15, from_tim15);
    impl_tim!(tim16, from_tim16);
    impl_tim!(tim17, from_tim17);

    /// Start the timer running by setting the CEN bit.
    pub fn start(&self) {
        modify_reg!(tim1, self.tim, CR1, CEN: Enabled);
    }

    /// Stop the timer running by clearing the CEN bit.
    pub fn stop(&self) {
        modify_reg!(tim1, self.tim, CR1, CEN: Disabled);
        write_reg!(tim1, self.tim, CNT, 0);
    }

    /// Start the timer in one-pulse mode for `ticks` period.
    pub fn start_oneshot(&self, ticks: u32) {
        write_reg!(tim1, self.tim, ARR, ticks);
        modify_reg!(tim1, self.tim, CR1, CEN: Enabled, OPM: Enabled);
    }

    /// Clear ISR flags.
    pub fn clear_uif(&self) {
        write_reg!(tim1, self.tim, SR, UIF: Clear);
    }

    /// Configure timer for use as HUB75E clock generation.
    ///
    /// Enables clock output on CH3 at f_tim/period frequency and 50% duty,
    /// with a DMA request generated every update.
    pub fn setup_hub_clk(&self, period: u32) {
        // Ensure timer is disabled and use defaults for CR1 and CR2.
        write_reg!(tim1, self.tim, CR1, CEN: Disabled);
        write_reg!(tim1, self.tim, CR2, 0);

        // Enable DMA requests on CC3 match.
        write_reg!(tim1, self.tim, DIER, CC3DE: 1);

        // In PWM mode 2, output is inactive while CNT<CCR3, giving us an idle-low
        // condition and a rising edge halfway through the timer period. The DMA
        // request is generated on the CC3 match at the rising edge, causing the
        // GPIOs to be updated about 15ns after the rising edge, well clear of
        // the 5ns hold time requirement.
        write_reg!(tim1, self.tim, CCMR2, OC3M: PwmMode2, CC3S: Output);

        // Enable CC3 output with active-high polarity.
        write_reg!(tim1, self.tim, CCER, CC3P: 0, CC3E: 1);

        // Don't prescale, run timer at full timer clock.
        write_reg!(tim1, self.tim, PSC, 0);

        // Set total period, which divides the timer clock.
        write_reg!(tim1, self.tim, ARR, period - 1);

        // Set compare to half the period for 50% duty cycle.
        write_reg!(tim1, self.tim, CCR3, period / 2);

        // Set main-output-enable.
        write_reg!(tim1, self.tim, BDTR, MOE: 1);

        // Generate an update to load the preloaded registers.
        write_reg!(tim1, self.tim, EGR, UG: Update);
    }

    /// Configure timer for use a HUB75E OE generation.
    ///
    /// Generates one-shot pulses on CH2 for a configurable period,
    /// with an interrupt request generated after each pulse.
    pub fn setup_hub_oe(&self) {
        // Ensure timer is disabled and enable one-pulse mode.
        write_reg!(tim1, self.tim, CR1, CEN: Disabled, OPM: Enabled);
        write_reg!(tim1, self.tim, CR2, 0);

        // Enable interrupt on update.
        write_reg!(tim1, self.tim, DIER, UIE: Enabled);

        // In PWM mode 1, CH2 is active while CNT<CCR2, thus the falling edge occurs
        // just after the counter starts, and then rises again after the one-shot pulse.
        write_reg!(tim1, self.tim, CCMR1, OC2M: PwmMode1, CC2S: Output);

        // Enable CC2 output with active-high polarity (since we use PWM mode 1,
        // it is active=high most of the time, as required for the nOE signal).
        write_reg!(tim1, self.tim, CCER, CC2P: 0, CC2E: 1);

        // Don't prescale, run timer at full timer clock.
        write_reg!(tim1, self.tim, PSC, 0);

        // Write CCR2 to 1 to trigger pulse just after starting the counter.
        write_reg!(tim1, self.tim, CCR2, 1);

        // Set main-output-enable.
        write_reg!(tim1, self.tim, BDTR, MOE: 1);
    }

    /// Configure timer for 10fps main loop timing.
    ///
    /// Enables interrupt generation.
    pub fn setup_tick(&self, psc: u32, period: u32) {
        // Ensure timer is disabled.
        write_reg!(tim1, self.tim, CR1, CEN: Disabled);
        write_reg!(tim1, self.tim, CR2, 0);

        // Enable interrupt on update.
        write_reg!(tim1, self.tim, DIER, UIE: Enabled);

        // Prescale by provided prescaler.
        write_reg!(tim1, self.tim, PSC, psc);

        // Set ARR to provided period.
        write_reg!(tim1, self.tim, ARR, period - 1);
    }

    /// Configure timer for LCD backlight PWM generation on CH2N.
    pub fn setup_lcd_pwm(&self, period: u32) {
        // Ensure timer is disabled and use defaults for CR1 and CR2.
        write_reg!(tim1, self.tim, CR1, CEN: Disabled);
        write_reg!(tim1, self.tim, CR2, 0);

        // Use PWM mode 1, with output active while CNT<CCR2.
        write_reg!(tim1, self.tim, CCMR1, OC2M: PwmMode1, CC2S: Output);

        // Enable CC2N output with active-high polarity.
        write_reg!(tim1, self.tim, CCER, CC2NP: 0, CC2NE: 1);

        // Don't prescale, run timer at full timer clock.
        write_reg!(tim1, self.tim, PSC, 0);

        // Set total period, which divides the timer clock.
        write_reg!(tim1, self.tim, ARR, period - 1);

        // Set initial duty cycle to 0.
        write_reg!(tim1, self.tim, CCR2, 0);

        // Set main-output-enable.
        write_reg!(tim1, self.tim, BDTR, MOE: 1);

        // Generate an update to load the preloaded registers.
        write_reg!(tim1, self.tim, EGR, UG: Update);

        // Start the PWM output.
        modify_reg!(tim1, self.tim, CR1, CEN: Enabled);
    }

    /// Set duty cycle for LCD backlight PWM.
    ///
    /// `duty` ranges from 0 to the `period` specified at setup.
    pub fn set_lcd_duty(&self, duty: u32) {
        write_reg!(tim1, self.tim, CCR2, duty);
    }

    /// Configure timer to measure CH2 input.
    ///
    /// Prescales timer by 50 and captures counter value on each input rising edge,
    /// so a 50Hz input gives a count of 60000. Triggers interrupt on each capture.
    pub fn setup_psc50_ti2(&self) {
        // Ensure timer is disabled and use defaults for CR1 and CR2.
        write_reg!(tim1, self.tim, CR1, CEN: Disabled);
        write_reg!(tim1, self.tim, CR2, 0);

        // Enable interrupt on CC2.
        write_reg!(tim1, self.tim, DIER, CC2IE: Enabled);

        // Set IC2 input mode: not filtered or prescaled, input with IC2=TI2.
        write_reg!(tim1, self.tim, CCMR1, CC2S: 0b01);

        // Prescale clock by 50 so that a 50Hz input does not overflow 16-bit counter.
        write_reg!(tim1, self.tim, PSC, 50 - 1);

        // Count up to 65535.
        write_reg!(tim1, self.tim, ARR, 0xFFFF);

        // Generate update event to reinitialise timer.
        write_reg!(tim1, self.tim, EGR, UG: Update);

        // Enable counter.
        write_reg!(tim1, self.tim, CR1, CEN: Enabled);

        // Enable TI2 input.
        write_reg!(tim1, self.tim, CCER, CC2E: 1);
    }

    /// Configure a TIM15 timer to measure LSE (TI1SEL=0b0100).
    ///
    /// Prescales LSE by 8 and captures the counter value on each divided-by-8 LSE rising edge,
    /// so expected value is about 36621.09 counts at a 4kHz capture rate. Drives DMA with each
    /// capture.
    pub fn setup_tim15_lse(&self) {
        // Ensure timer is disabled and use defaults for CR1 and CR2.
        write_reg!(tim1, self.tim, CR1, CEN: Disabled);
        write_reg!(tim1, self.tim, CR2, 0);

        // Enable DMA on CC1.
        write_reg!(tim1, self.tim, DIER, CC1DE: Enabled);

        // Set TI1 to LSE 0b0100.
        write_reg!(tim1, self.tim, TISEL, TI1SEL: 0b0100);

        // Set TI1 input mode: not filtered, prescale by 8, input with IC1=TI1.
        write_reg!(tim1, self.tim, CCMR1, IC1F: NoFilter, IC1PSC: 0b11, CC1S: 0b01);

        // No prescaler.
        write_reg!(tim1, self.tim, PSC, 0);

        // Count up to 65535.
        write_reg!(tim1, self.tim, ARR, 0xFFFF);

        // Generate update event to reinitialise timer.
        write_reg!(tim1, self.tim, EGR, UG: Update);

        // Enable counter.
        write_reg!(tim1, self.tim, CR1, CEN: Enabled);

        // Enable TI1 input.
        write_reg!(tim1, self.tim, CCER, CC1E: 1);
    }

    /// Return address of CCR1 register.
    pub fn ccr1(&self) -> u32 {
        &self.tim.CCR1 as *const _ as u32
    }

    /// Read current value in CC2 register.
    pub fn cc2(&self) -> u32 {
        read_reg!(tim1, self.tim, CCR2)
    }
}
