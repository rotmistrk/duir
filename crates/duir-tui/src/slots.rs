//! Slot identifiers for duir's 4-slot `TiledWorkspace` layout.

/// Identifies one of the four panel slots.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
#[repr(usize)]
#[allow(dead_code)]
pub enum SlotId {
    Left = 0,
    Center = 1,
    Right = 2,
    Bottom = 3,
}

pub const PANEL_COUNT: usize = 4;
