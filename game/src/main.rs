//! Light Show — desktop binary entry point. Android uses the separate
//! `#[bevy_main] fn main()` inside `lib.rs`, loaded as a `cdylib` by the
//! `game-activity` native glue instead of this binary.

fn main() {
    light_show::run();
}
