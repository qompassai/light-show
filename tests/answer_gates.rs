//! Answer-tag gates over the shipped level data.
//!
//! Two builds, two assertions — the same test proves whichever
//! property the build under test must have:
//!
//! - Keyed build (`LIGHT_SHOW_ANSWER_KEY` injected, matching the
//!   shipped tags): the correct answers verify, wrong answers and
//!   tampered tags are rejected.
//! - Keyless (or wrong-key) build: verification fails closed — no
//!   candidate verifies against any shipped tag, and the correct
//!   index cannot be recovered. This is the anti-scrape property:
//!   the public tree carries tags nobody can test candidates
//!   against without the key.
//!
//! The key is baked at compile time via `option_env!`; the game's
//! build.rs declares `rerun-if-env-changed`, so changing the
//! injection rebuilds the crate. Run the keyless half in a fresh
//! target dir with no key in the environment.

use light_show::answer_verify::{answer_key, verify_tag};
use light_show::level::{load_level, score_quiz};

#[test]
fn shipped_quiz_tags_gate_on_the_build_key() {
    let level = load_level(70); // lea1
    let quiz = level.quiz.as_ref().expect("lea1 must have a quiz");
    let first = &quiz.questions[0];

    match first.reveal_correct() {
        Some(correct) => {
            // Keyed branch: the matching key is compiled in.
            assert!(answer_key().is_some());
            assert!(first.verify_choice(correct));
            assert!(!first.verify_choice((correct + 1) % 4));
            // A tampered tag (one hex digit flipped) must not verify.
            let mut bytes = first.correct_tag.clone().into_bytes();
            bytes[0] = if bytes[0] == b'0' { b'1' } else { b'0' };
            let tampered = String::from_utf8(bytes).unwrap();
            assert!(!verify_tag("quiz", &first.id, &correct.to_string(), &tampered));
            // Every question in the level recovers and verifies.
            for q in &quiz.questions {
                let idx = q.reveal_correct().expect("keyed build must reveal");
                assert!(q.verify_choice(idx));
            }
            eprintln!("answer_gates: keyed branch (tags verify, tamper rejected)");
        }
        None => {
            // Fail-closed branch: no candidate verifies, nothing
            // recovers, and no fixed answer vector scores.
            assert!(
                (0..4).all(|c| !first.verify_choice(c)),
                "keyless build verified a candidate — fail-closed is broken"
            );
            for q in &quiz.questions {
                assert!(q.reveal_correct().is_none());
            }
            let zeros = vec![0usize; quiz.questions.len()];
            let (score, _) = score_quiz(&quiz.questions, &zeros);
            assert_eq!(score, 0, "keyless build scored a shipped quiz");
            eprintln!("answer_gates: fail-closed branch (nothing verifies)");
        }
    }
}
