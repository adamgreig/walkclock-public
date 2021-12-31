use core::fmt::Write;
use crate::name::Name;

/// Menu structure.
///
/// The menu consists of `N_CATEGORIES` categories, each of which contains `N_SETTINGS` settings.
///
/// The menu allows selecting a category, then selecting a setting in that category,
/// and finally adjusting that setting. Settings may be disabled to prevent them from
/// displaying, which allows categories to have fewer than `N_SETTINGS` actual settings.
///
/// Each setting may be a boolean on/off switch, a numeric `i16` with a specified minimum
/// and maximum value, or a choice from a selection of strings.
///
/// The menu state can be serialised to/from a slice of u16s, one per setting.
#[derive(Clone, Debug)]
pub struct Menu<const N_CATEGORIES: usize, const N_SETTINGS: usize> {
    index: usize,
    active: bool,
    category_selected: bool,
    categories: [Category<N_SETTINGS>; N_CATEGORIES],
}

#[derive(Copy, Clone, Debug)]
pub struct Category<const N_SETTINGS: usize> {
    name: Name,
    index: usize,
    setting_selected: bool,
    settings: [Setting; N_SETTINGS],
}

#[derive(Copy, Clone, Debug)]
pub struct Setting {
    name: Name,
    enabled: bool,
    value: Value,
}

#[derive(Copy, Clone, Debug)]
pub enum Value {
    OnOff(bool),
    Numeric {
        min: i16,
        max: i16,
        val: i16,
    },
    Choice {
        index: usize,
        choices: &'static [Name],
    },
}

impl<const N_CATEGORIES: usize, const N_SETTINGS: usize> Menu<N_CATEGORIES, N_SETTINGS> {
    pub const fn new(categories: [Category<N_SETTINGS>; N_CATEGORIES]) -> Self {
        Self {
            index: 0,
            active: false,
            category_selected: false,
            categories,
        }
    }

    pub fn active(&self) -> bool {
        self.active
    }

    pub fn category(&self, name: Name) -> Option<&Category<N_SETTINGS>> {
        self.categories.iter().find(|c| c.name() == name)
    }

    pub fn category_mut(&mut self, name: Name) -> Option<&mut Category<N_SETTINGS>> {
        self.categories.iter_mut().find(|c| c.name() == name)
    }

    pub fn category_name(&self) -> Name {
        self.categories[self.index].name()
    }

    pub fn category_selected(&self) -> bool {
        self.category_selected
    }

    pub fn setting_name(&self) -> Name {
        self.categories[self.index].setting_name()
    }

    pub fn setting_selected(&self) -> bool {
        self.categories[self.index].setting_selected()
    }

    pub fn render_value<W: Write>(&self, w: W) -> core::fmt::Result {
        self.categories[self.index].render_value(w)
    }

    pub fn serialise(&self, mut data: &mut [u16]) {
        for category in self.categories.iter() {
            let n = category.serialise(data);
            data = &mut data[n..];
        }
    }

    pub fn deserialise(&mut self, mut data: &[u16]) {
        for category in self.categories.iter_mut() {
            let n = category.deserialise(data);
            data = &data[n..];
        }
    }

    pub fn inc(&mut self) -> bool {
        if self.category_selected {
            self.categories[self.index].inc()
        } else {
            if self.index == N_CATEGORIES - 1 {
                self.index = 0;
            } else {
                self.index += 1;
            }
            false
        }
    }

    pub fn dec(&mut self) -> bool {
        if self.category_selected {
            self.categories[self.index].dec()
        } else {
            if self.index == 0 {
                self.index = N_CATEGORIES - 1;
            } else {
                self.index -= 1;
            }
            false
        }
    }

    pub fn enter(&mut self) {
        if self.active {
            if self.category_selected {
                self.categories[self.index].enter();
            } else {
                self.category_selected = true;
            }
        } else {
            self.active = true;
            self.category_selected = false;
        }
    }

    pub fn back(&mut self) {
        if self.category_selected {
            if self.setting_selected() {
                self.categories[self.index].back();
            } else {
                self.category_selected = false;
            }
        } else {
            self.active = false;
        }
    }
}

impl<const N_SETTINGS: usize> Category<N_SETTINGS> {
    pub const fn new(name: Name, settings: [Setting; N_SETTINGS]) -> Self {
        Self {
            name,
            index: 0,
            setting_selected: false,
            settings,
        }
    }

    pub const fn name(&self) -> Name {
        self.name
    }

    pub fn setting(&self, name: Name) -> Option<&Setting> {
        self.settings.iter().find(|s| s.name() == name)
    }

    pub fn setting_mut(&mut self, name: Name) -> Option<&mut Setting> {
        self.settings.iter_mut().find(|s| s.name() == name)
    }

    pub fn setting_name(&self) -> Name {
        self.settings[self.index].name()
    }

    pub fn setting_selected(&self) -> bool {
        self.setting_selected
    }

    pub fn setting_onoff(&self, name: Name) -> Option<bool> {
        self.setting(name).map(|s| s.onoff()).flatten()
    }

    #[allow(unused)]
    pub fn setting_set_onoff(&mut self, name: Name, v: bool) -> Option<()> {
        self.setting_mut(name).map(|s| s.set_onoff(v)).flatten()
    }

    pub fn setting_numeric(&self, name: Name) -> Option<i16> {
        self.setting(name).map(|s| s.numeric()).flatten()
    }

    pub fn setting_set_numeric(&mut self, name: Name, v: i16) -> Option<()> {
        self.setting_mut(name).map(|s| s.set_numeric(v)).flatten()
    }

    pub fn setting_choice(&self, name: Name) -> Option<Name> {
        self.setting(name).map(|s| s.choice()).flatten()
    }

    pub fn setting_set_choice(&mut self, name: Name, v: Name) -> Option<()> {
        self.setting_mut(name).map(|s| s.set_choice(v)).flatten()
    }

    pub fn render_value<W: Write>(&self, w: W) -> core::fmt::Result {
        self.settings[self.index].render(w)
    }

    pub fn setting_set_enabled(&mut self, name: Name, enabled: bool) -> Option<()> {
        self.setting_mut(name).map(|s| s.set_enabled(enabled))
    }

    pub fn setting_set_max(&mut self, name: Name, max: i16) -> Option<()> {
        self.setting_mut(name).map(|s| s.set_max(max)).flatten()
    }

    pub fn serialise(&self, data: &mut [u16]) -> usize {
        let mut data = data.iter_mut();
        let mut n_settings = 0;
        for setting in self.settings.iter() {
            if setting.name() != Name::Unused {
                if let Some(word) = data.next() {
                    *word = setting.serialise();
                    n_settings += 1;
                }
            }
        }
        n_settings
    }

    pub fn deserialise(&mut self, data: &[u16]) -> usize {
        let mut data = data.iter();
        let mut n_settings = 0;
        for setting in self.settings.iter_mut() {
            if setting.name() != Name::Unused {
                if let Some(word) = data.next() {
                    setting.deserialise(*word);
                    n_settings += 1;
                }
            }
        }
        n_settings
    }

    pub fn inc(&mut self) -> bool {
        if self.setting_selected {
            self.settings[self.index].inc();
            true
        } else {
            if self.index == N_SETTINGS - 1 {
                self.index = 0;
            } else {
                self.index += 1;
            }
            if !self.settings[self.index].enabled() {
                self.inc();
            }
            false
        }
    }

    pub fn dec(&mut self) -> bool {
        if self.setting_selected {
            self.settings[self.index].dec();
            true
        } else {
            if self.index == 0 {
                self.index = N_SETTINGS - 1;
            } else {
                self.index -= 1;
            }
            if !self.settings[self.index].enabled() {
                self.dec();
            }
            false
        }
    }

    pub fn enter(&mut self) {
        self.setting_selected = !self.setting_selected;
    }

    pub fn back(&mut self) {
        self.setting_selected = false;
    }
}

impl Setting {
    pub const fn new(name: Name, enabled: bool, value: Value) -> Self {
        Self {
            name,
            enabled,
            value,
        }
    }

    pub const fn new_onoff(name: Name, enabled: bool, value: bool) -> Self {
        Self::new(name, enabled, Value::OnOff(value))
    }

    pub const fn new_numeric(
        name: Name,
        enabled: bool,
        min: i16,
        max: i16,
        val: i16,
    ) -> Self {
        Self::new(name, enabled, Value::Numeric { min, max, val })
    }

    pub const fn new_choice(
        name: Name,
        enabled: bool,
        index: usize,
        choices: &'static [Name],
    ) -> Self {
        Self::new(name, enabled, Value::Choice { index, choices })
    }

    pub const fn new_disabled() -> Self {
        Self::new(Name::Unused, false, Value::OnOff(false))
    }

    pub const fn name(&self) -> Name {
        self.name
    }

    #[allow(unused)]
    pub fn value(&self) -> &Value {
        &self.value
    }

    pub fn onoff(&self) -> Option<bool> {
        if let Value::OnOff(b) = self.value {
            Some(b)
        } else {
            None
        }
    }

    #[allow(unused)]
    pub fn set_onoff(&mut self, v: bool) -> Option<()> {
        if let Value::OnOff(ref mut b) = self.value {
            *b = v;
            Some(())
        } else {
            None
        }
    }

    pub fn numeric(&self) -> Option<i16> {
        if let Value::Numeric { val, .. } = self.value {
            Some(val)
        } else {
            None
        }
    }

    pub fn set_numeric(&mut self, v: i16) -> Option<()> {
        if let Value::Numeric { min, max, ref mut val } = self.value {
            if v >= min && v <= max {
                *val = v;
                Some(())
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn choice(&self) -> Option<Name> {
        if let Value::Choice { index, choices } = self.value {
            Some(choices[index])
        } else {
            None
        }
    }

    pub fn set_choice(&mut self, v: Name) -> Option<()> {
        if let Value::Choice { ref mut index, choices } = self.value {
            if let Some(idx) = choices.iter().position(|c| *c == v) {
                *index = idx;
                return Some(());
            }
        }
        None
    }

    pub fn render<W: Write>(&self, mut w: W) -> core::fmt::Result {
        match &self.value {
            Value::OnOff(b) => if *b { write!(w, "On") } else { write!(w, "Off") },
            Value::Numeric { val, .. } => write!(w, "{}", val),
            Value::Choice { index, choices } => write!(w, "{}", choices[*index]),
        }
    }

    pub fn inc(&mut self) {
        match &mut self.value {
            Value::OnOff(b) => *b = !*b,
            Value::Numeric { min, max, val } => {
                if *val == *max {
                    *val = *min;
                } else {
                    *val += 1;
                }
            }
            Value::Choice { choices, index } => {
                if *index == choices.len() - 1 {
                    *index = 0;
                } else {
                    *index += 1;
                }
            }
        }
    }

    pub fn dec(&mut self) {
        match &mut self.value {
            Value::OnOff(b) => *b = !*b,
            Value::Numeric { min, max, val } => {
                if *val == *min {
                    *val = *max;
                } else {
                    *val -= 1;
                }
            }
            Value::Choice { choices, index } => {
                if *index == 0 {
                    *index = choices.len() - 1;
                } else {
                    *index -= 1;
                }
            }
        }
    }

    pub fn serialise(&self) -> u16 {
        match self.value {
            Value::OnOff(b) => b as u16,
            Value::Numeric { val, .. } => val as u16,
            Value::Choice { index, .. } => index as u16,
        }
    }

    pub fn deserialise(&mut self, word: u16) {
        match &mut self.value {
            Value::OnOff(b) => *b = word != 0,
            Value::Numeric { val, .. } => *val = word as i16,
            Value::Choice { index, .. } => *index = word as usize,
        }
    }

    pub fn set_max(&mut self, new_max: i16) -> Option<()> {
        if let Value::Numeric { max, val, .. } = &mut self.value {
            *max = new_max;
            if *val > new_max {
                *val = new_max;
            }
            Some(())
        } else {
            None
        }
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled
    }
}
