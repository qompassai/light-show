//! Static IPv4/IPv6 configuration console (Astra §2f).
//!
//! The player configures a device (the office printer) to the
//! assigned worksheet, per address family. Fields are chosen from
//! authored candidate lists — there is no free text anywhere, so a
//! selection is always a value the level authored (worksheet value
//! or a named trap). Apply produces one verdict per family via
//! `LevelDef::config_apply_verdict`; families never share a verdict
//! (dual-stack rule, §3): IPv4 passing says nothing about IPv6.
//!
//! Availability is the inventory rule only (§2f guardrail):
//! `address_availability` consults the allocations list and the
//! gateway — never the DHCP pool (the pool is printed on the
//! worksheet because the trap needs it visible: an in-pool address
//! allocated to someone else is still unavailable, and the
//! worksheet address stays right even though it sits in the pool
//! range the DHCP server would also hand out).

use bevy::prelude::*;

use crate::level::{ConfigFamily, LevelDef, StateStatus, StaticConfigDef};
use crate::states::GameState;
use crate::waifu::reactions::{ReactionInbox, ReactionTrigger};

/// One field of a family form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigField {
    Address,
    Prefix,
    Gateway,
    GatewayInterface,
    Dns,
}

/// Per-family form state: the current selections (seeded from the
/// level's authored `initial_*` — the pre-existing misconfiguration)
/// and the apply record.
#[derive(Debug, Clone)]
pub struct FamilyProgress {
    pub address: String,
    pub prefix_len: u32,
    pub gateway: String,
    pub gateway_interface: Option<String>,
    pub dns: String,
    pub applies: u32,
    pub rejected_applies: u32,
    pub passed: bool,
    pub last_feedback: Option<String>,
    pub mismatch_reaction_fired: bool,
}

impl FamilyProgress {
    /// Seed from the authored initial (mis)configuration.
    pub fn from_def(def: &StaticConfigDef) -> Self {
        Self {
            address: def.initial_address.clone(),
            prefix_len: def.initial_prefix_len,
            gateway: def.initial_gateway.clone(),
            gateway_interface: def.initial_gateway_interface.clone(),
            dns: def.initial_dns.clone(),
            applies: 0,
            rejected_applies: 0,
            passed: false,
            last_feedback: None,
            mismatch_reaction_fired: false,
        }
    }

    /// Cycle one field to its next authored candidate. Returns
    /// false for a field the family does not have (the V6-only
    /// gateway interface on a V4 form) or an empty candidate list —
    /// malformed data fails safe, never panics. Any change un-passes
    /// the family: a pass belongs to the applied selections.
    pub fn cycle(&mut self, def: &StaticConfigDef, field: ConfigField) -> bool {
        let changed = match field {
            ConfigField::Address => cycle_str(&mut self.address, &def.address_candidates),
            ConfigField::Prefix => cycle_u32(&mut self.prefix_len, &def.prefix_candidates),
            ConfigField::Gateway => cycle_str(&mut self.gateway, &def.gateway_candidates),
            ConfigField::GatewayInterface => {
                let Some(current) = self.gateway_interface.as_mut() else {
                    return false;
                };
                cycle_str(current, &def.interfaces)
            }
            ConfigField::Dns => cycle_str(&mut self.dns, &def.dns_candidates),
        };
        if changed {
            self.passed = false;
        }
        changed
    }

    /// Apply the current selections: the per-family verdict.
    /// `Ok` marks the family passed; `Err` carries the first
    /// failing field's feedback (§6 wording) and counts a rejection.
    pub fn apply(&mut self, def: &StaticConfigDef) -> Result<(), String> {
        self.applies = self.applies.saturating_add(1);
        let verdict = crate::level::config_apply_verdict(
            def,
            &self.address,
            self.prefix_len,
            &self.gateway,
            self.gateway_interface.as_deref(),
            &self.dns,
        );
        match verdict {
            Ok(()) => {
                self.passed = true;
                self.last_feedback = None;
                Ok(())
            }
            Err(feedback) => {
                self.rejected_applies = self.rejected_applies.saturating_add(1);
                self.last_feedback = Some(feedback.clone());
                Err(feedback)
            }
        }
    }

    /// The family's state line (§2a) — its own verdict, always.
    pub fn state(&self, def: &StaticConfigDef) -> (StateStatus, String) {
        let name = match def.family {
            ConfigFamily::V4 => "IPv4",
            ConfigFamily::V6 => "IPv6",
        };
        if self.passed {
            return (
                StateStatus::Pass,
                format!(
                    "Static {name} accepted — {}/{} via {}, DNS {}.",
                    self.address, self.prefix_len, self.gateway, self.dns
                ),
            );
        }
        if let Some(feedback) = &self.last_feedback {
            return (StateStatus::Fail, feedback.clone());
        }
        (
            StateStatus::Pending,
            format!("Static {name} not yet applied for {}.", def.device_label),
        )
    }
}

fn cycle_str(current: &mut String, candidates: &[String]) -> bool {
    if candidates.is_empty() {
        return false;
    }
    let next = candidates
        .iter()
        .position(|c| c == current)
        .map(|i| candidates[(i + 1) % candidates.len()].clone())
        .unwrap_or_else(|| candidates[0].clone());
    if next == *current && candidates.len() < 2 {
        return false;
    }
    *current = next;
    true
}

fn cycle_u32(current: &mut u32, candidates: &[u32]) -> bool {
    if candidates.is_empty() {
        return false;
    }
    let next = candidates
        .iter()
        .position(|c| c == current)
        .map(|i| candidates[(i + 1) % candidates.len()])
        .unwrap_or(candidates[0]);
    if next == *current && candidates.len() < 2 {
        return false;
    }
    *current = next;
    true
}

/// Player-side static-config state, one form per authored family.
#[derive(Resource, Debug, Default)]
pub struct ConfigProgress {
    pub v4: Option<FamilyProgress>,
    pub v6: Option<FamilyProgress>,
}

impl ConfigProgress {
    /// (Re)seed both forms from the level's authored blocks. Called
    /// on level entry; absent blocks leave the family unseeded.
    pub fn seed(&mut self, level: &LevelDef) {
        self.v4 = level
            .static_config_v4
            .as_ref()
            .map(FamilyProgress::from_def);
        self.v6 = level
            .static_config_v6
            .as_ref()
            .map(FamilyProgress::from_def);
    }

    pub fn family(&self, family: ConfigFamily) -> Option<&FamilyProgress> {
        match family {
            ConfigFamily::V4 => self.v4.as_ref(),
            ConfigFamily::V6 => self.v6.as_ref(),
        }
    }

    pub fn family_mut(&mut self, family: ConfigFamily) -> Option<&mut FamilyProgress> {
        match family {
            ConfigFamily::V4 => self.v4.as_mut(),
            ConfigFamily::V6 => self.v6.as_mut(),
        }
    }

    /// The gate fact for one family: seeded, applied, passed.
    pub fn family_passed(&self, family: ConfigFamily) -> bool {
        self.family(family).is_some_and(|f| f.passed)
    }
}

/// Marker for the config console root (despawn cleanup).
#[derive(Component)]
pub struct ConfigConsole;

/// Marker for config console buttons.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub struct ConfigButton(pub ConfigAction);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ConfigAction {
    Cycle(ConfigFamily, ConfigField),
    Apply(ConfigFamily),
}

/// Status text entity (single multi-line text, api-console idiom).
#[derive(Component)]
pub struct ConfigStatusText;

/// Static config console plugin.
pub struct ConfigConsolePlugin;

impl Plugin for ConfigConsolePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ConfigProgress>()
            .add_systems(OnEnter(GameState::Playing), seed_config_progress)
            .add_systems(
                Update,
                (handle_config_clicks, refresh_config_console).run_if(
                    in_state(GameState::Playing).or_else(in_state(GameState::OutageActive)),
                ),
            )
            .add_systems(OnEnter(GameState::Results), cleanup_config_console)
            .add_systems(OnEnter(GameState::MainMenu), cleanup_config_console);
    }
}

fn seed_config_progress(level: Option<Res<LevelDef>>, mut progress: ResMut<ConfigProgress>) {
    let Some(level) = level else {
        return;
    };
    progress.seed(&level);
}

fn family_name(family: ConfigFamily) -> &'static str {
    match family {
        ConfigFamily::V4 => "IPv4",
        ConfigFamily::V6 => "IPv6",
    }
}

/// The console's status block: both worksheets, both forms, both
/// verdicts — side by side so the dual-stack rule is visible.
/// Pure formatting (unit-tested).
pub fn status_text(level: &LevelDef, progress: &ConfigProgress) -> Option<String> {
    let mut out = String::new();
    for def in [&level.static_config_v4, &level.static_config_v6]
        .into_iter()
        .flatten()
    {
        let name = family_name(def.family);
        out.push_str(&format!(
            "{} worksheet — {}: address {}/{}, gateway {}",
            name,
            def.device_label,
            def.worksheet_address,
            def.worksheet_prefix_len,
            def.worksheet_gateway
        ));
        if let Some(iface) = &def.worksheet_gateway_interface {
            out.push_str(&format!(" (via {iface})"));
        }
        out.push_str(&format!(", DNS {}.\n", def.worksheet_dns));
        if let Some((lo, hi)) = &def.dhcp_pool {
            out.push_str(&format!(
                "  DHCP pool {lo} – {hi} (for reference — availability is the inventory below).\n"
            ));
        }
        for alloc in &def.allocations {
            out.push_str(&format!(
                "  inventory: {} → {}\n",
                alloc.address, alloc.holder
            ));
        }
        match progress.family(def.family) {
            Some(form) => {
                out.push_str(&format!(
                    "  form: {}/{} via {}",
                    form.address, form.prefix_len, form.gateway
                ));
                if let Some(iface) = &form.gateway_interface {
                    out.push_str(&format!(" (via {iface})"));
                }
                out.push_str(&format!(", DNS {}.\n", form.dns));
                if form.passed {
                    out.push_str(&format!("  {name} verdict: ACCEPTED.\n"));
                } else if let Some(feedback) = &form.last_feedback {
                    out.push_str(&format!("  {name} verdict: REJECTED — {feedback}\n"));
                } else {
                    out.push_str(&format!("  {name} verdict: not applied yet.\n"));
                }
            }
            None => out.push_str("  form not seeded.\n"),
        }
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

/// Spawn the config console (called from the `OnEnter(Playing)`
/// chain after `setup_level`). No-op without a config block.
pub fn setup_config_console(mut commands: Commands, level: Option<Res<LevelDef>>) {
    let Some(level) = level else {
        return;
    };
    let families: Vec<&StaticConfigDef> = [&level.static_config_v4, &level.static_config_v6]
        .into_iter()
        .flatten()
        .collect();
    if families.is_empty() {
        return;
    }
    let root = commands
        .spawn((
            ConfigConsole,
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(16.0),
                bottom: Val::Px(16.0),
                width: Val::Px(420.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.0),
                padding: UiRect::all(Val::Px(10.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.05, 0.07, 0.12, 0.92)),
        ))
        .id();
    let status = commands
        .spawn((
            ConfigStatusText,
            Text::new("Static configuration"),
            TextFont {
                font_size: FontSize::Px(13.0),
                ..default()
            },
            TextColor(Color::WHITE),
        ))
        .id();
    commands.entity(root).add_child(status);
    for def in families {
        let name = family_name(def.family);
        let fields = [
            (ConfigField::Address, "address"),
            (ConfigField::Prefix, "prefix"),
            (ConfigField::Gateway, "gateway"),
            (ConfigField::GatewayInterface, "gateway interface"),
            (ConfigField::Dns, "DNS"),
        ];
        for (field, label) in fields {
            if field == ConfigField::GatewayInterface && def.worksheet_gateway_interface.is_none() {
                continue;
            }
            let button = spawn_config_button(
                &mut commands,
                ConfigAction::Cycle(def.family, field),
                &format!("{name}: next {label}"),
            );
            commands.entity(root).add_child(button);
        }
        let apply = spawn_config_button(
            &mut commands,
            ConfigAction::Apply(def.family),
            &format!("Apply {name}"),
        );
        commands.entity(root).add_child(apply);
    }
}

fn spawn_config_button(commands: &mut Commands, action: ConfigAction, label: &str) -> Entity {
    commands
        .spawn((
            ConfigButton(action),
            Button,
            Node {
                padding: UiRect::all(Val::Px(6.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.16, 0.2, 0.32, 1.0)),
        ))
        .with_child((
            Text::new(label.to_string()),
            TextFont {
                font_size: FontSize::Px(13.0),
                ..default()
            },
            TextColor(Color::WHITE),
        ))
        .id()
}

fn handle_config_clicks(
    level: Option<Res<LevelDef>>,
    mut progress: ResMut<ConfigProgress>,
    buttons: Query<
        (&Interaction, &ConfigButton),
        (
            Changed<Interaction>,
            Without<super::identification::IdentificationButton>,
            Without<super::jumper::JumperButton>,
            Without<super::survey::SurveyButton>,
            Without<super::workbench::WorkbenchButton>,
            Without<super::api_console::ApiButton>,
            Without<super::triage_console::TriageButton>,
        ),
    >,
    mut reaction_inbox: Option<ResMut<ReactionInbox>>,
) {
    let Some(level) = level else {
        return;
    };
    for (interaction, button) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match button.0 {
            ConfigAction::Cycle(family, field) => {
                let Some(def) = (match family {
                    ConfigFamily::V4 => &level.static_config_v4,
                    ConfigFamily::V6 => &level.static_config_v6,
                })
                .as_ref() else {
                    continue;
                };
                if let Some(form) = progress.family_mut(family) {
                    form.cycle(def, field);
                }
            }
            ConfigAction::Apply(family) => {
                let Some(def) = (match family {
                    ConfigFamily::V4 => &level.static_config_v4,
                    ConfigFamily::V6 => &level.static_config_v6,
                })
                .as_ref() else {
                    continue;
                };
                let mut fire_mismatch = false;
                if let Some(form) = progress.family_mut(family) {
                    if form.apply(def).is_err() && !form.mismatch_reaction_fired {
                        form.mismatch_reaction_fired = true;
                        fire_mismatch = true;
                    }
                }
                if fire_mismatch {
                    if let Some(mut inbox) = reaction_inbox.as_deref_mut() {
                        inbox.push(ReactionTrigger::ConfigMismatch);
                    }
                }
            }
        }
    }
}

fn refresh_config_console(
    level: Option<Res<LevelDef>>,
    progress: Res<ConfigProgress>,
    mut query: Query<&mut Text, With<ConfigStatusText>>,
) {
    let Some(level) = level else {
        return;
    };
    let Some(text) = status_text(&level, &progress) else {
        return;
    };
    for mut t in &mut query {
        if t.0 != text {
            t.0.clone_from(&text);
        }
    }
}

fn cleanup_config_console(mut commands: Commands, query: Query<Entity, With<ConfigConsole>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::level::LEVEL_SOURCES;

    fn load(id: &str) -> LevelDef {
        LEVEL_SOURCES
            .iter()
            .map(|s| serde_json::from_str(s).unwrap())
            .find(|l: &LevelDef| l.id == id)
            .unwrap()
    }

    /// Drive a form to the worksheet values by cycling (bounded:
    /// candidate lists are short; 8 cycles per field is generous).
    fn drive_to_worksheet(form: &mut FamilyProgress, def: &StaticConfigDef) {
        for _ in 0..8 {
            if form.address != def.worksheet_address {
                form.cycle(def, ConfigField::Address);
            }
            if form.prefix_len != def.worksheet_prefix_len {
                form.cycle(def, ConfigField::Prefix);
            }
            if form.gateway != def.worksheet_gateway {
                form.cycle(def, ConfigField::Gateway);
            }
            if form.gateway_interface != def.worksheet_gateway_interface {
                form.cycle(def, ConfigField::GatewayInterface);
            }
            if form.dns != def.worksheet_dns {
                form.cycle(def, ConfigField::Dns);
            }
        }
    }

    #[test]
    fn v4_initial_config_is_rejected_then_worksheet_values_pass() {
        let level = load("m1l5");
        let def = level.static_config_v4.as_ref().expect("m1l5 authors V4");
        let mut form = FamilyProgress::from_def(def);
        // The authored initial fault: wrong gateway (.254).
        let err = form.apply(def).expect_err("initial config must fail");
        assert!(err.contains("gateway"), "{err}");
        assert_eq!(form.rejected_applies, 1);
        drive_to_worksheet(&mut form, def);
        assert!(form.apply(def).is_ok());
        assert!(form.passed);
        let (status, _) = form.state(def);
        assert_eq!(status, StateStatus::Pass);
    }

    #[test]
    fn dhcp_pool_traps_reject_by_inventory_never_by_pool() {
        let level = load("m1l5");
        let def = level.static_config_v4.as_ref().unwrap();
        // In-pool but allocated to the Office PC: rejected, and the
        // feedback names availability (inventory), not the pool.
        let mut form = FamilyProgress::from_def(def);
        drive_to_worksheet(&mut form, def);
        form.address = "192.168.40.105".to_string();
        let err = form.apply(def).expect_err("allocated address must fail");
        assert!(err.contains("not available"), "{err}");
        assert!(err.contains("allocated to another device"), "{err}");
        // Out-of-pool but allocated to the Label printer: STILL
        // rejected by inventory — the pool is not the rule.
        form.address = "192.168.40.25".to_string();
        let err = form.apply(def).expect_err("allocated address must fail");
        assert!(err.contains("not available"), "{err}");
        // The gateway itself as an address: collision.
        form.address = def.worksheet_gateway.clone();
        let err = form.apply(def).expect_err("gateway collision must fail");
        assert!(err.contains("not available"), "{err}");
        // And the worksheet address — inside the pool range and
        // allocated to the printer itself — passes availability.
        assert!(crate::level::address_availability(def, &def.worksheet_address).is_ok());
    }

    #[test]
    fn v6_wrong_interface_is_the_named_verdict() {
        // Adversarial (spec §6): every field matches the worksheet
        // except the gateway's outgoing interface — the verdict is
        // exactly the wrong-interface string, nothing vaguer.
        let level = load("m1l7");
        let def = level.static_config_v6.as_ref().expect("m1l7 authors V6");
        let mut form = FamilyProgress::from_def(def);
        assert_eq!(
            form.gateway_interface.as_deref(),
            Some("management"),
            "the authored initial fault is the wrong interface"
        );
        drive_to_worksheet(&mut form, def);
        form.gateway_interface = Some("management".to_string());
        let err = form.apply(def).expect_err("wrong interface must fail");
        assert_eq!(err, "IPv6 default route uses the wrong outgoing interface.");
        // The global-looking gateway trap fails by mismatch.
        form.gateway_interface = def.worksheet_gateway_interface.clone();
        form.gateway = "2001:db8:40:1::1".to_string();
        let err = form.apply(def).expect_err("global gateway trap must fail");
        assert!(err.contains("gateway does not match"), "{err}");
        // Fully correct passes — and only the V6 form moved.
        drive_to_worksheet(&mut form, def);
        assert!(form.apply(def).is_ok());
        assert!(form.passed);
    }

    #[test]
    fn families_verdict_independently() {
        // The dual-stack rule (§3): passing one family changes
        // nothing about the other. m1l10 carries both (slice 8);
        // here both shipped blocks are seeded into one progress and
        // driven on separate levels' data to prove the machinery.
        let v4_level = load("m1l5");
        let v6_level = load("m1l7");
        let mut both = v4_level.clone();
        both.static_config_v6 = v6_level.static_config_v6.clone();
        let mut progress = ConfigProgress::default();
        progress.seed(&both);
        let v4def = both.static_config_v4.as_ref().unwrap().clone();
        let mut form = progress.v4.take().unwrap();
        drive_to_worksheet(&mut form, &v4def);
        assert!(form.apply(&v4def).is_ok());
        progress.v4 = Some(form);
        assert!(progress.family_passed(ConfigFamily::V4));
        assert!(
            !progress.family_passed(ConfigFamily::V6),
            "IPv4 passing must not verify IPv6"
        );
        let (v6_status, _) = progress
            .v6
            .as_ref()
            .unwrap()
            .state(both.static_config_v6.as_ref().unwrap());
        assert_eq!(v6_status, StateStatus::Pending);
    }

    #[test]
    fn changing_a_passed_form_unpasses_it() {
        // A pass belongs to the applied selections: touch any field
        // afterward and the family must be re-applied.
        let level = load("m1l5");
        let def = level.static_config_v4.as_ref().unwrap();
        let mut form = FamilyProgress::from_def(def);
        drive_to_worksheet(&mut form, def);
        assert!(form.apply(def).is_ok());
        assert!(form.passed);
        assert!(form.cycle(def, ConfigField::Dns));
        assert!(!form.passed);
    }

    #[test]
    fn malformed_candidates_fail_safe() {
        // Adversarial: empty candidate lists and cross-family
        // fields are rejected, never panics or phantom changes.
        let level = load("m1l5");
        let mut def = level.static_config_v4.as_ref().unwrap().clone();
        def.address_candidates = vec![];
        let mut form = FamilyProgress::from_def(&def);
        assert!(!form.cycle(&def, ConfigField::Address));
        // The V6-only interface field does not exist on a V4 form.
        assert!(!form.cycle(&def, ConfigField::GatewayInterface));
    }
}
