//! Custom WGSL shaders: glow, fiber pulse flow, background atmosphere,
//! and transition wipes.
//!
//! All rendering previously went through Bevy's built-in pipelines. These
//! four materials add the light-fantasy juice a fiber-optic game needs:
//!
//! * [`GlowMaterial`] — radial bloom glow with a breathing pulse, for win
//!   bursts and node highlights.
//! * [`PulseMaterial`] — light traveling along fibers as a smooth,
//!   resolution-independent flow (replaces frame-based sprite pulses
//!   where continuous flow reads better).
//! * [`AtmosphereMaterial`] — fullscreen animated gradient + vignette +
//!   subtle scanlines behind everything.
//! * [`WipeMaterial`] — directional transition wipe driven by the fade
//!   state machine (a [`UiMaterial`] so it covers UI too).
//!
//! Portability: WGSL is restricted to constructs valid on Vulkan, Metal,
//! D3D12, and GLES3 (no dynamic indexing, no exotic builtins). Every
//! shader is validated by naga in `tests/shader_render.rs`.

use bevy::asset::{Asset, UntypedAssetId, VisitAssetDependencies};
use bevy::prelude::*;
use bevy::reflect::TypePath;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;
use bevy::sprite_render::{AlphaMode2d, Material2d, Material2dPlugin, MeshMaterial2d};
use bevy::ui_render::prelude::UiMaterialPlugin;
use bevy::ui_render::ui_material::{MaterialNode, UiMaterial};

// ---------------------------------------------------------------------------
// Glow
// ---------------------------------------------------------------------------

/// Uniforms for `assets/shaders/glow.wgsl`. 16-byte aligned: vec4 (16)
/// + 4×f32 (16) = 32 bytes.
#[derive(Debug, Clone, ShaderType)]
pub struct GlowSettings {
    pub color: Vec4,
    pub time: f32,
    pub intensity: f32,
    pub pulse_speed: f32,
    pub pulse_amount: f32,
}

impl Default for GlowSettings {
    fn default() -> Self {
        Self {
            color: Vec4::ONE,
            time: 0.0,
            intensity: 1.0,
            pulse_speed: 3.0,
            pulse_amount: 0.25,
        }
    }
}

/// Radial glow with a breathing pulse.
#[derive(AsBindGroup, Debug, Clone)]
pub struct GlowMaterial {
    #[uniform(0)]
    pub settings: GlowSettings,
}

impl Material2d for GlowMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/glow.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

// ---------------------------------------------------------------------------
// Pulse
// ---------------------------------------------------------------------------

/// Uniforms for `assets/shaders/pulse.wgsl`: vec4 (16) + 4×f32 (16).
#[derive(Debug, Clone, ShaderType)]
pub struct PulseSettings {
    pub color: Vec4,
    pub time: f32,
    pub speed: f32,
    pub width: f32,
    pub base_brightness: f32,
}

impl Default for PulseSettings {
    fn default() -> Self {
        Self {
            color: Vec4::ONE,
            time: 0.0,
            speed: 0.8,
            width: 0.08,
            base_brightness: 0.25,
        }
    }
}

/// Light pulse traveling along a fiber (quad U axis = fiber direction).
#[derive(AsBindGroup, Debug, Clone)]
pub struct PulseMaterial {
    #[uniform(0)]
    pub settings: PulseSettings,
}

impl Material2d for PulseMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/pulse.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

// ---------------------------------------------------------------------------
// Atmosphere
// ---------------------------------------------------------------------------

/// Uniforms for `assets/shaders/atmosphere.wgsl`: 2×vec4 (32) + 4×f32 (16).
#[derive(Debug, Clone, ShaderType)]
pub struct AtmosphereSettings {
    pub top_color: Vec4,
    pub bottom_color: Vec4,
    pub time: f32,
    pub scanline_strength: f32,
    pub vignette_strength: f32,
    pub drift_speed: f32,
}

impl Default for AtmosphereSettings {
    fn default() -> Self {
        Self {
            top_color: Vec4::new(0.03, 0.05, 0.10, 1.0),
            bottom_color: Vec4::new(0.01, 0.02, 0.05, 1.0),
            time: 0.0,
            scanline_strength: 0.05,
            vignette_strength: 0.45,
            drift_speed: 0.25,
        }
    }
}

/// Fullscreen animated background: gradient + vignette + scanlines.
#[derive(AsBindGroup, Debug, Clone)]
pub struct AtmosphereMaterial {
    #[uniform(0)]
    pub settings: AtmosphereSettings,
}

impl Material2d for AtmosphereMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/atmosphere.wgsl".into()
    }
}

// ---------------------------------------------------------------------------
// Wipe (UI material — covers UI as well as the 2D world)
// ---------------------------------------------------------------------------

/// Uniforms for `assets/shaders/wipe.wgsl`: vec4 (16) + 4×f32 (16).
#[derive(Debug, Clone, ShaderType)]
pub struct WipeSettings {
    pub color: Vec4,
    pub progress: f32,
    pub softness: f32,
    pub direction_x: f32,
    pub direction_y: f32,
}

impl Default for WipeSettings {
    fn default() -> Self {
        Self {
            color: Vec4::new(0.0, 0.0, 0.0, 1.0),
            progress: 0.0,
            softness: 0.12,
            // Diagonal sweep: dream-transition feel.
            direction_x: 0.7,
            direction_y: 0.7,
        }
    }
}

/// Directional transition wipe. Applied to the fullscreen fade overlay
/// node; `progress` is driven by the fade state machine in anim.rs.
#[derive(AsBindGroup, Debug, Clone)]
pub struct WipeMaterial {
    #[uniform(0)]
    pub settings: WipeSettings,
}

impl UiMaterial for WipeMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/wipe.wgsl".into()
    }
}

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

/// Registers all shader materials and ticks their `time` uniforms.
pub struct ShaderPlugin;

impl Plugin for ShaderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(Material2dPlugin::<GlowMaterial>::default());
        app.add_plugins(Material2dPlugin::<PulseMaterial>::default());
        app.add_plugins(Material2dPlugin::<AtmosphereMaterial>::default());
        app.add_plugins(UiMaterialPlugin::<WipeMaterial>::default());
        app.add_systems(Update, tick_shader_time);
    }
}

/// Advance the `time` uniform on every shader material each frame.
fn tick_shader_time(
    time: Res<Time>,
    mut glows: ResMut<Assets<GlowMaterial>>,
    mut pulses: ResMut<Assets<PulseMaterial>>,
    mut atmospheres: ResMut<Assets<AtmosphereMaterial>>,
) {
    let dt = time.delta_secs();
    for (_, m) in glows.iter_mut() {
        m.settings.time += dt;
    }
    for (_, m) in pulses.iter_mut() {
        m.settings.time += dt;
    }
    for (_, m) in atmospheres.iter_mut() {
        m.settings.time += dt;
    }
}

/// Spawn a glow quad at `position` with the given color and size.
/// Returns the entity for later despawn/fade.
pub fn spawn_glow(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<GlowMaterial>,
    position: Vec3,
    color: LinearRgba,
    size_px: f32,
) -> Entity {
    commands
        .spawn((
            Mesh2d(meshes.add(Rectangle::new(size_px, size_px))),
            MeshMaterial2d(materials.add(GlowMaterial {
                settings: GlowSettings {
                    color: color.to_vec4(),
                    ..Default::default()
                },
            })),
            Transform::from_translation(position),
        ))
        .id()
}

/// Marker for the UI fade node using [`WipeMaterial`].
#[derive(Debug, Component)]
pub struct WipeOverlay;

/// Spawn the fullscreen wipe overlay node (replaces the flat fade).
/// Kept transparent (`progress = 0`) until the fade driver moves it.
pub fn spawn_wipe_overlay(commands: &mut Commands, materials: &mut Assets<WipeMaterial>) -> Entity {
    commands
        .spawn((
            WipeOverlay,
            MaterialNode(materials.add(WipeMaterial {
                settings: WipeSettings::default(),
            })),
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                ..default()
            },
            // Above all UI: the wipe must cover every menu and dialog.
            GlobalZIndex(i32::MAX),
        ))
        .id()
}

// `Asset` is a methodless marker trait; our materials hold no asset
// handles so dependency visiting is a no-op. Manual impls avoid the
// derive-macro resolution issue across the bevy re-export boundary.
impl VisitAssetDependencies for GlowMaterial {
    fn visit_dependencies(&self, _visit: &mut impl FnMut(UntypedAssetId)) {}
}
impl Asset for GlowMaterial {}
impl VisitAssetDependencies for PulseMaterial {
    fn visit_dependencies(&self, _visit: &mut impl FnMut(UntypedAssetId)) {}
}
impl Asset for PulseMaterial {}
impl VisitAssetDependencies for AtmosphereMaterial {
    fn visit_dependencies(&self, _visit: &mut impl FnMut(UntypedAssetId)) {}
}
impl Asset for AtmosphereMaterial {}
impl VisitAssetDependencies for WipeMaterial {
    fn visit_dependencies(&self, _visit: &mut impl FnMut(UntypedAssetId)) {}
}
impl Asset for WipeMaterial {}

impl TypePath for GlowMaterial {
    fn type_path() -> &'static str {
        "light_show::shaders::GlowMaterial"
    }
    fn short_type_path() -> &'static str {
        "GlowMaterial"
    }
}
impl TypePath for PulseMaterial {
    fn type_path() -> &'static str {
        "light_show::shaders::PulseMaterial"
    }
    fn short_type_path() -> &'static str {
        "PulseMaterial"
    }
}
impl TypePath for AtmosphereMaterial {
    fn type_path() -> &'static str {
        "light_show::shaders::AtmosphereMaterial"
    }
    fn short_type_path() -> &'static str {
        "AtmosphereMaterial"
    }
}
impl TypePath for WipeMaterial {
    fn type_path() -> &'static str {
        "light_show::shaders::WipeMaterial"
    }
    fn short_type_path() -> &'static str {
        "WipeMaterial"
    }
}
