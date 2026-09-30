//! Sprite-sheet layout constants shared by all four companions' 96x192
//! full-body pixel-art frames (Séraphine, Ondine, Linka, Lattice all use
//! the same 6-mood-row x 4-frame layout). See `docs/ART_STYLE.md` for the
//! full art direction brief given to artists/AI-art tooling.

pub const FRAME_W: u32 = 96;
pub const FRAME_H: u32 = 192;
pub const FRAMES_PER_ROW: u32 = 4;
pub const MOOD_ROWS: u32 = 6; // idle, blush, wink, pout, celebrate, alarmed

/// Builds the shared grid layout every companion's sprite sheet uses.
/// One layout asset is reused across all four companions since they all
/// follow the identical 96x192, 4-column, 6-row convention.
pub fn atlas_layout() -> bevy::sprite::TextureAtlasLayout {
    bevy::sprite::TextureAtlasLayout::from_grid(
        bevy::math::UVec2::new(FRAME_W, FRAME_H),
        FRAMES_PER_ROW,
        MOOD_ROWS,
        None,
        None,
    )
}

/// Flattens a (mood row, animation frame) pair into the linear index the
/// `TextureAtlas` component needs to show the correct cell of the sheet.
pub fn atlas_index(mood_row: usize, frame: usize) -> usize {
    mood_row * FRAMES_PER_ROW as usize + frame
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atlas_index_is_row_major() {
        assert_eq!(atlas_index(0, 0), 0);
        assert_eq!(atlas_index(0, 3), 3);
        assert_eq!(atlas_index(1, 0), 4);
        assert_eq!(atlas_index(5, 3), 23);
    }

    #[test]
    fn atlas_layout_matches_the_documented_grid() {
        let layout = atlas_layout();
        assert_eq!(
            layout.size,
            bevy::math::UVec2::new(FRAME_W * FRAMES_PER_ROW, FRAME_H * MOOD_ROWS)
        );
        assert_eq!(layout.textures.len(), (FRAMES_PER_ROW * MOOD_ROWS) as usize);
    }
}
