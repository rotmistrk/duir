/// Bitflags for boolean state in App.
#[derive(Debug, Clone, Copy, Default)]
pub struct AppFlags(u8);

impl AppFlags {
    const SHOULD_QUIT: u8 = 1;
    const AUTOSAVE_GLOBAL: u8 = 1 << 1;
    const PENDING_DELETE: u8 = 1 << 2;
    const FILTER_COMMITTED_EXCLUDE: u8 = 1 << 3;
    const KIRO_TAB_FOCUSED: u8 = 1 << 4;
    const ZOOMED: u8 = 1 << 5;
    const KBD_MAC: u8 = 1 << 6;

    const fn has(self, flag: u8) -> bool {
        self.0 & flag != 0
    }

    const fn set(&mut self, flag: u8, value: bool) {
        if value {
            self.0 |= flag;
        } else {
            self.0 &= !flag;
        }
    }

    #[must_use]
    pub const fn should_quit(self) -> bool {
        self.has(Self::SHOULD_QUIT)
    }
    #[must_use]
    pub const fn autosave_global(self) -> bool {
        self.has(Self::AUTOSAVE_GLOBAL)
    }
    #[must_use]
    pub const fn pending_delete(self) -> bool {
        self.has(Self::PENDING_DELETE)
    }
    #[must_use]
    pub const fn filter_committed_exclude(self) -> bool {
        self.has(Self::FILTER_COMMITTED_EXCLUDE)
    }
    #[must_use]
    pub const fn kiro_tab_focused(self) -> bool {
        self.has(Self::KIRO_TAB_FOCUSED)
    }
    #[must_use]
    pub const fn zoomed(self) -> bool {
        self.has(Self::ZOOMED)
    }
    #[must_use]
    pub const fn kbd_mac(self) -> bool {
        self.has(Self::KBD_MAC)
    }

    pub const fn set_should_quit(&mut self, v: bool) {
        self.set(Self::SHOULD_QUIT, v);
    }
    pub const fn set_autosave_global(&mut self, v: bool) {
        self.set(Self::AUTOSAVE_GLOBAL, v);
    }
    pub const fn set_pending_delete(&mut self, v: bool) {
        self.set(Self::PENDING_DELETE, v);
    }
    pub const fn set_filter_committed_exclude(&mut self, v: bool) {
        self.set(Self::FILTER_COMMITTED_EXCLUDE, v);
    }
    pub const fn set_kiro_tab_focused(&mut self, v: bool) {
        self.set(Self::KIRO_TAB_FOCUSED, v);
    }
    pub const fn set_zoomed(&mut self, v: bool) {
        self.set(Self::ZOOMED, v);
    }
    pub const fn set_kbd_mac(&mut self, v: bool) {
        self.set(Self::KBD_MAC, v);
    }
}
