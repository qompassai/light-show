# ls-gates: deterministic build / lint / test / safety / debugger gates.
#
# Mirrors the repo's bacon jobs and then some:
#   cargo build --workspace --locked
#   cargo clippy --workspace --all-targets --locked -- -D warnings
#   cargo fmt --all -- --check
#   cargo test --workspace --locked
# plus cargo-deny, cargo-audit, and the lldb-dap breakpoint smoke test.
# Every step is transcribed into reports/gates-<UTC timestamp>.md.

ls_check_deps cargo rustc clippy-driver cargo-fmt git python3 lldb-dap cargo-deny
ls_start_report "gates"

ls_section "Build and test gates"
ls_step "cargo build --workspace --locked" cargo build --workspace --locked
ls_step "cargo clippy --workspace --all-targets --locked -- -D warnings" \
    cargo clippy --workspace --all-targets --locked -- -D warnings
ls_step "cargo fmt --all -- --check" cargo fmt --all -- --check
ls_step "cargo test --workspace --locked" cargo test --workspace --locked

ls_safety_checks
ls_debug_smoke
ls_verify_clean

echo "report: ${LS_REPORT}"
if [ "${LS_FAILED}" -ne 0 ]; then
    echo "GATES FAILED - see ${LS_REPORT}" >&2
fi
exit "${LS_FAILED}"
