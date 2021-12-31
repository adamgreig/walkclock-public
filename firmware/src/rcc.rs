use stm32ral::{rcc, pwr, flash, read_reg, write_reg, modify_reg};

/// Frequencies for each clock in the system, in Hz.
#[derive(Copy, Clone, Debug)]
pub struct Clocks {
    pub sys_ck: u32,
    pub ahb_ck: u32,
    pub apb_ck: u32,
    pub tim_ck: u32,
    pub rtc_ck: u32,
    pub spi4_ck: u32,
    pub uart8_ck: u32,
}

/// Configure device clocks.
///
/// Uses a 25MHz HSE crystal oscillator and 32.768kHz LSE crystal oscillator.
pub fn setup(rcc: rcc::Instance, pwr: pwr::Instance, flash: flash::Instance) -> Clocks {
    // Initialise power control.
    write_reg!(pwr, pwr, CR3, SCUEN: 1, LDOEN: 1, BYPASS: 0);
    while read_reg!(pwr, pwr, CSR1, ACTVOSRDY == 0) {}

    // Set VOS to Scale 2, where f_cpu max 300MHz, f_hclk max 150MHz, f_pclk max 75MHz.
    // Note: scale 3 is 0b01, scale 2 is 0b10, scale 1 is 0b11.
    write_reg!(pwr, pwr, D3CR, VOS: 0b10);
    while read_reg!(pwr, pwr, D3CR, VOSRDY == 0) {}

    // Set flash wait states.
    // For VOS2 and hclk=120MHz, use WS=2.
    write_reg!(flash, flash, ACR, WRHIGHFREQ: 0b01, LATENCY: 2);

    // Ensure HSI is enabled, stable, and in use.
    // All other clocks and PLLs are explicitly disabled.
    // This is the reset state, we just enforce it before
    // setting up the other clocks in case of a soft reset.
    write_reg!(rcc, rcc, CR, HSION: On, HSEON: Off, PLL1ON: Off);
    while read_reg!(rcc, rcc, CR, HSIRDY != Ready) {}
    while read_reg!(rcc, rcc, CR, HSIDIVF != Propagated) {}
    write_reg!(rcc, rcc, CFGR, SW: HSI);
    while read_reg!(rcc, rcc, CFGR, SWS != HSI) {}

    // Enable HSE.
    modify_reg!(rcc, rcc, CR, HSEON: On);
    while read_reg!(rcc, rcc, CR, HSERDY != Ready) {}

    // Configure and enable PLL1.
    // Input is 25M hse_ck, DIVM=25 to give ref1_ck=1MHz.
    // DIVN1=300 for vco1ck=301M, DIVP1=0 for pll1_p_ck=301M.
    // Note cheeky 301MHz instead of 300MHz which it turns out shifts the HUB75E-related
    // harmonic content just out of the GPS spectrum where it was otherwise stopping us
    // from getting a lock (!). For the rest of the firmware we pretend it's 300MHz.
    write_reg!(rcc, rcc, PLLCKSELR, PLLSRC: HSE, DIVM1: 25);
    write_reg!(rcc, rcc, PLLCFGR, DIVP1EN: Enabled, PLL1RGE: Range1, PLL1VCOSEL: MediumVCO);
    write_reg!(rcc, rcc, PLL1DIVR, DIVP1: 0, DIVN1: 301 - 1);
    modify_reg!(rcc, rcc, CR, PLL1ON: On);
    while read_reg!(rcc, rcc, CR, PLL1RDY != Ready) {}

    // Configure clock dividers.
    // D1CPRE=/1 -> d1_ck = sys_ck = 300MHz
    //   HPRE=/2 -> rcc_hclk3 = rcc_aclk = sys_d1cpre_ck/2 = 150MHz
    // D1PPRE=/2 -> rcc_pclk3 = rcc_hclk3/2 = 75MHz
    write_reg!(rcc, rcc, D1CFGR, D1CPRE: Div1, HPRE: Div2, D1PPRE: Div2);
    // D2PPRE1=/2 -> rcc_pclk1 = rcc_hclk1 / 2 = 75MHz
    // D2PPRE2=/2 -> rcc_pclk2 = rcc_hclk1 / 2 = 75MHz
    write_reg!(rcc, rcc, D2CFGR, D2PPRE1: Div2, D2PPRE2: Div2);
    // D3PPRE=/2 -> rcc_pclk4 = rcc_hclk4 / 2 = 75MHz
    write_reg!(rcc, rcc, D3CFGR, D3PPRE: Div2);

    // Swap system clock source to PLL1.
    modify_reg!(rcc, rcc, CFGR, SW: PLL1);
    while read_reg!(rcc, rcc, CFGR, SWS != PLL1) {}

    // Configure and enable peripheral clocks.

    // SPI45SEL left at default so SPI4 uses rcc_pclk1=75M.
    write_reg!(rcc, rcc, D2CCIP1R, SPI45SEL: APB);

    // Disable write-protect on backup domain (RTC and backup RAM).
    modify_reg!(pwr, pwr, CR1, DBP: 1);

    // Enable RTC, clocked from LSE. Requires DBP=1 in PWR_CR1.
    write_reg!(rcc, rcc, BDCR, RTCEN: Enabled, RTCSEL: LSE, LSEON: On);
    while read_reg!(rcc, rcc, BDCR, LSERDY != Ready) {}

    // Enable AHB peripherals: DMA1, DMA2, SRAM1, SRAM2, JPEG, DMA2D, MDMA, GPIOs.
    write_reg!(rcc, rcc, AHB1ENR, DMA1EN: Enabled, DMA2EN: Enabled);
    write_reg!(rcc, rcc, AHB2ENR, SRAM1EN: Enabled, SRAM2EN: Enabled);
    write_reg!(rcc, rcc, AHB3ENR, JPGDECEN: Enabled, DMA2DEN: Enabled, MDMAEN: Enabled);
    write_reg!(rcc, rcc, AHB4ENR, GPIOAEN: Enabled, GPIOBEN: Enabled, GPIOCEN: Enabled,
                                  GPIODEN: Enabled, GPIOEEN: Enabled);

    // Enable APB1 peripherals: TIM2, TIM3, TIM4, TIM12, UART8
    write_reg!(rcc, rcc, APB1LENR, TIM2EN: Enabled, TIM3EN: Enabled, TIM4EN: Enabled,
                                   TIM12EN: Enabled, UART8EN: Enabled);

    // Enable APB2 peripherals: TIM1, TIM15, SPI4
    write_reg!(rcc, rcc, APB2ENR, TIM1EN: Enabled, TIM15EN: Enabled, SPI4EN: Enabled);

    // Enable APB4 peripherals: RTC APB, SYSCFG
    write_reg!(rcc, rcc, APB4ENR, RTCAPBEN: Enabled, SYSCFGEN: Enabled);

    // Optional: enable LSE or HSE on MCO1 for calibration
    if cfg!(feature = "mco1_lse") {
        modify_reg!(rcc, rcc, CFGR, MCO1: LSE, MCO1PRE: 1);
    } else if cfg!(feature = "mco1_hse") {
        modify_reg!(rcc, rcc, CFGR, MCO1: HSE, MCO1PRE: 1);
    }

    // Return generated clock frequencies for easy reference elsewhere.
    Clocks {
        sys_ck: 300_000_000,
        ahb_ck: 150_000_000,
        apb_ck: 75_000_000,
        tim_ck: 150_000_000,
        rtc_ck: 32_768,
        uart8_ck: 75_000_000,
        spi4_ck: 75_000_000,
    }
}
