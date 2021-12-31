use crate::gpio::Switches as GPIOSwitches;

struct Switch {
    on_time: u16,
    first_repeat: u16,
    next_repeat: u16,
}

pub struct Switches {
    gpio: GPIOSwitches,
    enter: Switch,
    qr: Switch,
    display: Switch,
    back: Switch,
    left: Switch,
    right: Switch,
}

impl Switch {
    /// Create a new Switch manager, which will return active on the
    /// first cycle where the switch is pressed, again on the `first_repeat` cycle,
    /// and then every `next_repeat` cycles thereafter.
    pub const fn new(first_repeat: u16, next_repeat: u16) -> Self {
        Switch { on_time: 0, first_repeat, next_repeat }
    }

    /// Update with the current state of the switch, `true` if pressed.
    pub fn update(&mut self, state: bool) {
        if state {
            self.on_time = self.on_time.saturating_add(1);
        } else {
            self.on_time = 0;
        }
    }

    /// Poll to see if the switch should be considered active this cycle.
    pub fn poll(&self) -> bool {
        if self.on_time == 1 {
            true
        } else if self.on_time >= self.first_repeat {
            (self.on_time - self.first_repeat) % self.next_repeat == 0
        } else {
            false
        }
    }
}

impl Switches {
    /// Create a new Switches manager, with all switches sharing the same `first_repeat`
    /// and `next_repeat` values.
    pub const fn new(gpio: GPIOSwitches, first_repeat: u16, next_repeat: u16) -> Self {
        Switches {
            gpio,
            enter: Switch::new(first_repeat, next_repeat),
            qr: Switch::new(first_repeat, next_repeat),
            display: Switch::new(first_repeat, next_repeat),
            back: Switch::new(first_repeat, next_repeat),
            left: Switch::new(first_repeat, next_repeat),
            right: Switch::new(first_repeat, next_repeat),
        }
    }

    /// Update all contained switches using the GPIO values.
    ///
    /// GPIO inputs are assumed to be active low.
    pub fn update(&mut self) {
        self.enter.update(!self.gpio.enter.get());
        self.qr.update(!self.gpio.qr.get());
        self.display.update(!self.gpio.display.get());
        self.back.update(!self.gpio.back.get());
        self.left.update(!self.gpio.left.get());
        self.right.update(!self.gpio.right.get());
    }

    /// Get state of enter button.
    pub fn enter(&self) -> bool {
        self.enter.poll()
    }

    /// Get state of QR button.
    pub fn qr(&self) -> bool {
        self.qr.poll()
    }

    /// Get state of display button.
    pub fn display(&self) -> bool {
        self.display.poll()
    }

    /// Get state of back button.
    pub fn back(&self) -> bool {
        self.back.poll()
    }

    /// Get state of left button.
    pub fn left(&self) -> bool {
        self.left.poll()
    }

    /// Get state of right button.
    pub fn right(&self) -> bool {
        self.right.poll()
    }
}
