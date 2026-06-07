//! Slot identifiers for duir's 3-slot `TiledWorkspace` layout.

/// Identifies one of the three panel slots.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
#[repr(usize)]
pub enum SlotId {
    Left = 0,
    Center = 1,
    Right = 2,
}

pub const PANEL_COUNT: usize = 3;
