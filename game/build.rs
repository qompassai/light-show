//! Build script: the answer key is read via `option_env!`, which
//! cargo does not track on its own — without this, a cached build
//! could silently keep a key (or its absence) from an earlier
//! invocation. Rebuild the crate whenever the injection changes.

fn main() {
    println!("cargo::rerun-if-env-changed=LIGHT_SHOW_ANSWER_KEY");
}
