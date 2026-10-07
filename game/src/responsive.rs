//! Responsive layout: every screen fits the window it actually got.
//!
//! Why this exists: the game is designed on a 720x1280 portrait
//! canvas, but it launches wherever the player launches it — a phone
//! in portrait, a desktop window in landscape, a resized window at
//! any aspect in between. The screens were built as if the window
//! were always the design canvas, and Bevy's `ImageNode` in its
//! default `NodeImageMode::Auto` paints a *contain* fit (the whole
//! image, letterboxed), so a full-window backdrop node only covers
//! the window when the aspects happen to match. Everywhere else the
//! artwork shrank to a centered band and the flat root background
//! showed at the corners — the picker's "black corner".
//!
//! This module owns the fix, in three parts:
//!
//! * [`cover_fit`] — pure cover-fit geometry (scale to cover,
//!   centered, overflow cropped symmetrically). Unit-tested.
//! * [`ArtBackdrop`] + [`ResponsivePlugin`] — mark any full-window
//!   backdrop image node; a single system keeps it cover-fitted to
//!   the real primary window, on spawn, on texture load, and on
//!   resize. Title backdrops additionally swap between the portrait
//!   and landscape title artworks by window shape
//!   ([`choose_title_art`]), falling back to the portrait art —
//!   which always ships — whenever the landscape file is absent.
//! * Screens keep their own layouts otherwise; the board camera's
//!   per-window fit lives in `board::board_framing` (re-applied on
//!   resize by `states::playing`), and edge-anchored UI (the
//!   dialogue pill) already reflows by construction.

use bevy::asset::LoadState;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

/// Asset path of the portrait title artwork (720x1280, one heroine
/// per corner). Ships unconditionally: it is the fallback for every
/// window shape and every doubt about the landscape variant.
pub const TITLE_ART_PORTRAIT_PATH: &str = "sprites/ui/title_artwork.png";
/// Asset path of the landscape title artwork (same four-corner
/// composition, wide canvas). Selected only when the window is
/// landscape-shaped *and* this file is actually present; produced
/// as a companion to the portrait original.
pub const TITLE_ART_LANDSCAPE_PATH: &str = "sprites/ui/title_artwork_landscape.png";

/// Window aspect (width / height) at or above which a window counts
/// as landscape-shaped for title-art selection. Square counts as
/// landscape: the wide composition crops more gracefully than the
/// tall one at 1:1 (pinned by the 1024x1024 render check).
pub const LANDSCAPE_ASPECT_MIN: f32 = 1.0;

/// Which title artwork a window shape calls for.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TitleArt {
    Portrait,
    Landscape,
}

impl TitleArt {
    /// The asset path this variant loads from.
    pub fn asset_path(self) -> &'static str {
        match self {
            TitleArt::Portrait => TITLE_ART_PORTRAIT_PATH,
            TitleArt::Landscape => TITLE_ART_LANDSCAPE_PATH,
        }
    }
}

/// Whether the landscape title artwork can be used, tracked from
/// the `AssetServer` load state of a probe load. `Unknown` is the
/// honest state until the load resolves; callers must treat it as
/// "not yet", never as "present".
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ArtAvailability {
    Unknown,
    Available,
    Missing,
}

/// The title artwork a bare aspect ratio asks for, ignoring
/// availability. Non-finite or negative aspects (degenerate window)
/// ask for portrait — the design-canvas orientation.
pub fn title_art_for_aspect(aspect: f32) -> TitleArt {
    if aspect.is_finite() && aspect >= LANDSCAPE_ASPECT_MIN {
        TitleArt::Landscape
    } else {
        TitleArt::Portrait
    }
}

/// The title artwork to display in a `window`-sized window: the
/// shape's request, gated on the landscape file being available.
/// The portrait art ships unconditionally, so it needs no gate and
/// is the answer to every unresolved probe.
pub fn choose_title_art(window: Vec2, landscape: ArtAvailability) -> TitleArt {
    let aspect = if window.y > 0.0 {
        window.x / window.y
    } else {
        0.0
    };
    match title_art_for_aspect(aspect) {
        TitleArt::Landscape if landscape == ArtAvailability::Available => TitleArt::Landscape,
        _ => TitleArt::Portrait,
    }
}

/// Cover-fit geometry for painting an image over a window: the size
/// to draw at, and the node's top-left offset relative to the
/// window's top-left. The image is scaled by the *larger* of the
/// two axis ratios (aspect preserved) and centered, so it covers
/// the window completely and the overflow crops symmetrically —
/// `offset` is component-wise `<= 0`, `size >= window`.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct CoverFit {
    pub size: Vec2,
    pub offset: Vec2,
}

/// Computes the cover fit of `image` over `window`. `None` when
/// either size is degenerate (zero, negative, or non-finite) — the
/// caller leaves the node alone that frame rather than painting
/// garbage geometry.
pub fn cover_fit(window: Vec2, image: Vec2) -> Option<CoverFit> {
    if !window.is_finite()
        || !image.is_finite()
        || window.x <= 0.0
        || window.y <= 0.0
        || image.x <= 0.0
        || image.y <= 0.0
    {
        return None;
    }
    let scale = (window.x / image.x).max(window.y / image.y);
    let size = image * scale;
    Some(CoverFit {
        size,
        offset: (window - size) / 2.0,
    })
}

/// Which artwork an [`ArtBackdrop`] node paints.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BackdropKind {
    /// The title artwork: portrait/landscape chosen by window shape
    /// via [`choose_title_art`]. Used by the menu and the picker.
    Title,
    /// One fixed image (e.g. the Warehouse interior): fit only,
    /// never swapped.
    Static,
}

/// Marks a full-window UI backdrop image node for cover fitting.
/// Attach next to the node's `ImageNode` (switched to
/// `NodeImageMode::Stretch` at spawn: the fit system owns the node
/// geometry, so the texture must map to the node rect exactly).
/// The applied-* fields are the fit system's change cache — they
/// keep the per-frame check O(1) per backdrop and leave the node
/// untouched when nothing relevant changed.
#[derive(Component)]
pub struct ArtBackdrop {
    kind: BackdropKind,
    applied_window: Vec2,
    applied_image: Option<AssetId<Image>>,
    applied_art: Option<TitleArt>,
}

impl ArtBackdrop {
    /// A title backdrop: art swapped by window shape, cover-fitted.
    pub fn title() -> Self {
        Self {
            kind: BackdropKind::Title,
            applied_window: Vec2::ZERO,
            applied_image: None,
            applied_art: None,
        }
    }

    /// A fixed-image backdrop: cover-fitted, never swapped.
    pub fn fixed() -> Self {
        Self {
            kind: BackdropKind::Static,
            applied_window: Vec2::ZERO,
            applied_image: None,
            applied_art: None,
        }
    }
}

/// The landscape title-art probe: one speculative load whose load
/// state is the availability oracle. Probing through the
/// `AssetServer` (rather than a filesystem check) keeps the answer
/// correct on Android, where assets live inside the APK.
#[derive(Resource)]
struct LandscapeArtProbe {
    handle: Handle<Image>,
    availability: ArtAvailability,
}

/// Keeps every [`ArtBackdrop`] node cover-fitted to the primary
/// window, swapping title art by window shape. Install once, in
/// `build_app`; the systems are a no-op in worlds without windows
/// or backdrops (headless tests).
pub struct ResponsivePlugin;

impl Plugin for ResponsivePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, probe_landscape_art)
            .add_systems(Update, fit_art_backdrops);
    }
}

fn probe_landscape_art(mut commands: Commands, asset_server: Res<AssetServer>) {
    let handle = asset_server.load(TITLE_ART_LANDSCAPE_PATH);
    commands.insert_resource(LandscapeArtProbe {
        handle,
        availability: ArtAvailability::Unknown,
    });
}

/// Advances the probe's availability from the load state. Resolved
/// states are sticky: once the file is known present or absent the
/// answer cannot change under a running game.
fn refresh_probe(probe: &mut LandscapeArtProbe, asset_server: &AssetServer) {
    if probe.availability != ArtAvailability::Unknown {
        return;
    }
    probe.availability = match asset_server.get_load_state(probe.handle.id()) {
        Some(LoadState::Loaded) => ArtAvailability::Available,
        Some(LoadState::Failed(_)) => ArtAvailability::Missing,
        _ => ArtAvailability::Unknown,
    };
}

/// The fit system. For each backdrop: pick the image it should
/// show (title art by shape + availability; static art is whatever
/// the screen spawned), wait until that texture has actually
/// loaded (its size is the fit input), then — only if the window,
/// the image, or the chosen art changed since the last application
/// — write the cover geometry into the node. Runs every frame;
/// the cache check keeps steady-state cost at one query walk over
/// the (at most three) live backdrops.
fn fit_art_backdrops(
    windows: Query<&Window, With<PrimaryWindow>>,
    asset_server: Res<AssetServer>,
    images: Res<Assets<Image>>,
    probe: Option<ResMut<LandscapeArtProbe>>,
    mut backdrops: Query<(&mut Node, &mut ImageNode, &mut ArtBackdrop)>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let window_size = Vec2::new(window.width(), window.height());
    if window_size.x <= 0.0 || window_size.y <= 0.0 {
        return;
    }
    let mut probe = probe;
    if let Some(probe) = probe.as_deref_mut() {
        refresh_probe(probe, &asset_server);
    }
    let availability = probe
        .as_ref()
        .map_or(ArtAvailability::Unknown, |probe| probe.availability);

    for (mut node, mut image_node, mut backdrop) in &mut backdrops {
        let chosen_art = match backdrop.kind {
            BackdropKind::Title => Some(choose_title_art(window_size, availability)),
            BackdropKind::Static => None,
        };
        let desired: Handle<Image> = match chosen_art {
            Some(art) => asset_server.load(art.asset_path()),
            None => image_node.image.clone(),
        };
        let Some(image) = images.get(&desired) else {
            // Texture not decoded yet (first frames, or a variant
            // swap mid-load): keep the current geometry; a later
            // frame applies the fit once the size is known.
            continue;
        };
        let Some(fit) = cover_fit(window_size, image.size().as_vec2()) else {
            continue;
        };
        if backdrop.applied_window == window_size
            && backdrop.applied_image == Some(desired.id())
            && backdrop.applied_art == chosen_art
        {
            continue;
        }
        image_node.image = desired.clone();
        node.width = Val::Px(fit.size.x);
        node.height = Val::Px(fit.size.y);
        node.left = Val::Px(fit.offset.x);
        node.top = Val::Px(fit.offset.y);
        backdrop.applied_window = window_size;
        backdrop.applied_image = Some(desired.id());
        backdrop.applied_art = chosen_art;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::asset::RenderAssetUsages;
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
    use bevy::window::WindowResolution;

    const EPS: f32 = 0.05;

    fn assert_vec2_near(actual: Vec2, expected: Vec2, label: &str) {
        assert!(
            (actual.x - expected.x).abs() <= EPS && (actual.y - expected.y).abs() <= EPS,
            "{label}: expected {expected:?}, got {actual:?}"
        );
    }

    // --- title-art aspect selection (validation) ---

    #[test]
    fn aspect_at_and_above_square_asks_for_landscape() {
        assert_eq!(title_art_for_aspect(16.0 / 9.0), TitleArt::Landscape);
        assert_eq!(title_art_for_aspect(21.0 / 9.0), TitleArt::Landscape);
        assert_eq!(title_art_for_aspect(1.0), TitleArt::Landscape);
    }

    #[test]
    fn aspect_below_square_asks_for_portrait() {
        assert_eq!(title_art_for_aspect(9.0 / 16.0), TitleArt::Portrait);
        assert_eq!(title_art_for_aspect(9.0 / 20.0), TitleArt::Portrait);
        assert_eq!(title_art_for_aspect(0.999), TitleArt::Portrait);
    }

    // --- title-art aspect selection (adversarial) ---

    #[test]
    fn degenerate_aspects_ask_for_portrait() {
        assert_eq!(title_art_for_aspect(0.0), TitleArt::Portrait);
        assert_eq!(title_art_for_aspect(-2.0), TitleArt::Portrait);
        assert_eq!(title_art_for_aspect(f32::NAN), TitleArt::Portrait);
        assert_eq!(title_art_for_aspect(f32::INFINITY), TitleArt::Portrait);
    }

    #[test]
    fn landscape_requires_availability() {
        let wide = Vec2::new(1920.0, 1080.0);
        assert_eq!(
            choose_title_art(wide, ArtAvailability::Available),
            TitleArt::Landscape
        );
        // The fallback contract: an unresolved or missing landscape
        // file can never blank the title screen.
        assert_eq!(
            choose_title_art(wide, ArtAvailability::Unknown),
            TitleArt::Portrait
        );
        assert_eq!(
            choose_title_art(wide, ArtAvailability::Missing),
            TitleArt::Portrait
        );
    }

    #[test]
    fn portrait_shapes_stay_portrait_even_when_landscape_exists() {
        let tall = Vec2::new(1080.0, 2400.0);
        assert_eq!(
            choose_title_art(tall, ArtAvailability::Available),
            TitleArt::Portrait
        );
        assert_eq!(
            choose_title_art(Vec2::ZERO, ArtAvailability::Available),
            TitleArt::Portrait
        );
        assert_eq!(
            choose_title_art(Vec2::new(100.0, 0.0), ArtAvailability::Available),
            TitleArt::Portrait
        );
    }

    // --- cover fit (validation) ---

    #[test]
    fn cover_fit_portrait_art_in_landscape_window_crops_top_bottom() {
        // The picker's black-corner case: 720x1280 art, 1920x1080
        // window. Width ratio 2.667 wins; height overflows.
        let fit = cover_fit(Vec2::new(1920.0, 1080.0), Vec2::new(720.0, 1280.0))
            .expect("valid sizes fit");
        assert_vec2_near(fit.size, Vec2::new(1920.0, 3413.33), "size");
        assert_vec2_near(fit.offset, Vec2::new(0.0, -1166.67), "offset");
    }

    #[test]
    fn cover_fit_portrait_art_in_taller_phone_window_crops_sides() {
        // 1080x2400 (9:20) is taller than the art's 9:16.
        let fit = cover_fit(Vec2::new(1080.0, 2400.0), Vec2::new(720.0, 1280.0))
            .expect("valid sizes fit");
        assert_vec2_near(fit.size, Vec2::new(1350.0, 2400.0), "size");
        assert_vec2_near(fit.offset, Vec2::new(-135.0, 0.0), "offset");
    }

    #[test]
    fn cover_fit_landscape_art_in_portrait_window_crops_sides() {
        // The Warehouse case: 2096x1184 interior, 720x1280 window.
        let fit = cover_fit(Vec2::new(720.0, 1280.0), Vec2::new(2096.0, 1184.0))
            .expect("valid sizes fit");
        assert_vec2_near(fit.size, Vec2::new(2265.95, 1280.0), "size");
        assert_vec2_near(fit.offset, Vec2::new(-772.97, 0.0), "offset");
    }

    #[test]
    fn cover_fit_matching_aspect_is_identity() {
        let fit =
            cover_fit(Vec2::new(720.0, 1280.0), Vec2::new(720.0, 1280.0)).expect("valid sizes fit");
        assert_vec2_near(fit.size, Vec2::new(720.0, 1280.0), "size");
        assert_vec2_near(fit.offset, Vec2::ZERO, "offset");
    }

    #[test]
    fn cover_fit_upscales_small_art_and_downscales_large_art() {
        let up = cover_fit(Vec2::new(2560.0, 1440.0), Vec2::new(720.0, 1280.0))
            .expect("valid sizes fit");
        assert_vec2_near(up.size, Vec2::new(2560.0, 4551.11), "upscaled size");
        let down = cover_fit(Vec2::new(720.0, 1600.0), Vec2::new(2096.0, 1184.0))
            .expect("valid sizes fit");
        assert!(down.size.x >= 720.0 && down.size.y >= 1600.0);
    }

    // --- cover fit (adversarial) ---

    #[test]
    fn cover_fit_rejects_degenerate_sizes() {
        assert_eq!(cover_fit(Vec2::ZERO, Vec2::new(720.0, 1280.0)), None);
        assert_eq!(cover_fit(Vec2::new(1920.0, 1080.0), Vec2::ZERO), None);
        assert_eq!(
            cover_fit(Vec2::new(-100.0, 1080.0), Vec2::new(720.0, 1280.0)),
            None
        );
        assert_eq!(
            cover_fit(Vec2::new(f32::NAN, 1080.0), Vec2::new(720.0, 1280.0)),
            None
        );
        assert_eq!(
            cover_fit(Vec2::new(1920.0, 1080.0), Vec2::new(720.0, f32::INFINITY)),
            None
        );
    }

    #[test]
    fn cover_fit_invariants_hold_across_the_dimension_matrix() {
        let windows = [
            Vec2::new(1920.0, 1080.0),
            Vec2::new(2560.0, 1440.0),
            Vec2::new(1080.0, 2400.0),
            Vec2::new(720.0, 1600.0),
            Vec2::new(1024.0, 1024.0),
            Vec2::new(3440.0, 1440.0),
            Vec2::new(800.0, 600.0),
        ];
        let images = [Vec2::new(720.0, 1280.0), Vec2::new(2096.0, 1184.0)];
        for window in windows {
            for image in images {
                let fit = cover_fit(window, image).expect("matrix sizes are valid");
                // Covers: never smaller than the window on either axis.
                assert!(fit.size.x >= window.x - EPS, "{window:?} x {image:?}");
                assert!(fit.size.y >= window.y - EPS, "{window:?} x {image:?}");
                // Centered crop: offsets never positive, and the far
                // edge reaches at least the window's far edge.
                assert!(fit.offset.x <= EPS && fit.offset.y <= EPS);
                assert!(fit.offset.x + fit.size.x >= window.x - EPS);
                assert!(fit.offset.y + fit.size.y >= window.y - EPS);
                // Aspect preserved.
                let drawn_aspect = fit.size.x / fit.size.y;
                let image_aspect = image.x / image.y;
                assert!((drawn_aspect - image_aspect).abs() <= 0.001);
            }
        }
    }

    // --- the fit system, headless (integration) ---

    fn solid_image(width: u32, height: u32) -> Image {
        Image::new_fill(
            Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            &[0, 0, 0, 255],
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::default(),
        )
    }

    fn fit_test_app(window: Vec2) -> (App, Entity) {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            bevy::asset::AssetPlugin::default(),
            ResponsivePlugin,
        ));
        app.init_asset::<Image>();
        let window_entity = app
            .world_mut()
            .spawn((
                Window {
                    resolution: WindowResolution::new(window.x as u32, window.y as u32),
                    ..default()
                },
                PrimaryWindow,
            ))
            .id();
        (app, window_entity)
    }

    fn spawn_backdrop(app: &mut App, image: Handle<Image>, backdrop: ArtBackdrop) -> Entity {
        app.world_mut()
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                ImageNode::new(image).with_mode(NodeImageMode::Stretch),
                backdrop,
            ))
            .id()
    }

    fn node_of(app: &App, entity: Entity) -> Node {
        app.world().get::<Node>(entity).expect("node").clone()
    }

    fn assert_node_fit(node: &Node, size: Vec2, offset: Vec2) {
        let (Val::Px(w), Val::Px(h), Val::Px(l), Val::Px(t)) =
            (node.width, node.height, node.left, node.top)
        else {
            panic!("node geometry must be px after fit: {node:?}");
        };
        assert_vec2_near(Vec2::new(w, h), size, "fitted size");
        assert_vec2_near(Vec2::new(l, t), offset, "fitted offset");
    }

    #[test]
    fn fit_system_cover_fits_a_static_backdrop() {
        let (mut app, _window) = fit_test_app(Vec2::new(1920.0, 1080.0));
        let handle = app
            .world_mut()
            .resource_mut::<Assets<Image>>()
            .add(solid_image(720, 1280));
        let backdrop = spawn_backdrop(&mut app, handle, ArtBackdrop::fixed());
        app.update();
        app.update();
        assert_node_fit(
            &node_of(&app, backdrop),
            Vec2::new(1920.0, 3413.33),
            Vec2::new(0.0, -1166.67),
        );
    }

    #[test]
    fn fit_system_refits_on_window_resize() {
        let (mut app, window_entity) = fit_test_app(Vec2::new(1920.0, 1080.0));
        let handle = app
            .world_mut()
            .resource_mut::<Assets<Image>>()
            .add(solid_image(720, 1280));
        let backdrop = spawn_backdrop(&mut app, handle, ArtBackdrop::fixed());
        app.update();
        app.update();
        // The player drags the window to a tall phone shape: the
        // backdrop must follow without a state change or restart.
        app.world_mut()
            .get_mut::<Window>(window_entity)
            .expect("window")
            .resolution = WindowResolution::new(1080, 2400);
        app.update();
        assert_node_fit(
            &node_of(&app, backdrop),
            Vec2::new(1350.0, 2400.0),
            Vec2::new(-135.0, 0.0),
        );
    }

    #[test]
    fn fit_system_title_backdrop_falls_back_to_portrait_while_probe_unresolved() {
        // Headless, the landscape probe never resolves to Available
        // (there is no file to load), so even in a wide window the
        // title backdrop must be cover-fitted with the portrait art —
        // never left at its spawn geometry, never blanked.
        let (mut app, _window) = fit_test_app(Vec2::new(1920.0, 1080.0));
        // The real title spawn loads the portrait path; mirror that
        // exactly, then stand in for the decoded texture under the
        // same handle so the fit system sees a loaded 720x1280 image.
        let portrait: Handle<Image> = app
            .world()
            .resource::<AssetServer>()
            .load(TITLE_ART_PORTRAIT_PATH);
        app.world_mut()
            .resource_mut::<Assets<Image>>()
            .insert(portrait.id(), solid_image(720, 1280))
            .expect("uuid handle insert cannot fail");
        let backdrop = spawn_backdrop(&mut app, portrait.clone(), ArtBackdrop::title());
        app.update();
        app.update();
        assert_node_fit(
            &node_of(&app, backdrop),
            Vec2::new(1920.0, 3413.33),
            Vec2::new(0.0, -1166.67),
        );
        let image_node = app.world().get::<ImageNode>(backdrop).expect("image node");
        assert_eq!(image_node.image.id(), portrait.id());
    }
}
