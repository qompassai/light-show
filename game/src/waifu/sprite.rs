//! Sprite-sheet layout constants shared by all four companions' 64x64
//! pixel-art frames (Séraphine, Ondine, Linka, Lattice all use the same
//! 6-mood-row x 4-frame layout). See `docs/ART_STYLE.md` for the full art
//! direction brief given to artists/AI-art tooling.

pub const FRAME_SIZE_PX: u32 = 64;
pub const FRAMES_PER_ROW: u32 = 4;
pub const MOOD_ROWS: u32 = 6; // idle, blush, wink, pout, celebrate, alarmed
