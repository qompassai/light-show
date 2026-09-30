# ls-bench-backends: primo-local render-backend benchmark matrix.
#
# PRIMO-LOCAL ONLY. This app is intentionally NOT part of `nix flake check`:
# it needs a display server (Xvfb), a built binary, and real GPU/GL drivers.
# Run it on primo via: nix run .#bench-backends
#
# Runs the deterministic bench harness (--bench-frames) once per reachable
# render backend (Vulkan, GL) and prints a comparison table. The bench
# harness itself is dev-only (#[cfg(debug_assertions)]) and changes nothing
# about the game when its flags are absent.
#
# Every run is transcribed into reports/bench-backends-<UTC timestamp>.md.

ls_check_deps cargo rustc git xdpyinfo
ls_start_report "bench-backends"

ls_section "Prerequisites"
# The bench needs a display. Reuse Xvfb :99 if it is up (the clips/bench
# workflow keeps one there); otherwise fail fast with a clear message
# instead of trying to start a new one (new Xvfbs fail when /tmp is full).
if DISPLAY=:99 xdpyinfo >/dev/null 2>&1; then
    ls_result "Xvfb :99 reachable" PASS ""
else
    ls_result "Xvfb :99 reachable" FAIL "no X server on :99"
    echo "No X server on :99. Start one first (e.g. Xvfb :99 &)." >&2
    echo "See docs/BACKENDS.md for the bench setup." >&2
    exit 1
fi

ls_section "Build (debug, bench harness is dev-only)"
ls_step "cargo build --locked" cargo build --locked

ls_section "Benchmark matrix"
bench_frames=300
bench_runs=3
for backend in vulkan gl; do
    for i in $(seq 1 "${bench_runs}"); do
        if [ "${backend}" = "vulkan" ]; then
            bench_out="$(DISPLAY=:99 ./target/debug/light-show \
                --bench-frames "${bench_frames}" 2>&1 | grep -a '^BENCH ' || true)"
        else
            bench_out="$(DISPLAY=:99 ./target/debug/light-show \
                --bench-frames "${bench_frames}" --bench-backend gl 2>&1 | grep -a '^BENCH ' || true)"
        fi
        # Fail the app if the bench line is missing (binary crashed/hung).
        if [ -z "${bench_out}" ]; then
            ls_result "bench ${backend} run ${i}" FAIL "no BENCH line in output"
            echo "bench produced no BENCH line (backend=${backend} run=${i})" >&2
            exit 1
        fi
        ls_result "bench ${backend} run ${i}" PASS "${bench_out}"
    done
done

ls_section "Notes"
cat >> "${LS_REPORT}" <<'NOTES'
- Numbers are Xvfb-local (no real display) and debug-build; they compare
  backends against each other, not against release or on-device performance.
- The GL backend here is Mesa llvmpipe (software rasterizer), not real GL
  hardware. Vulkan uses the discrete NVIDIA GPU via the host driver.
- Android numbers are NOT collected here; see docs/BACKENDS.md for why
  `dumpsys gfxinfo` does not apply to this NativeActivity/Vulkan game.
NOTES

ls_verify_clean
echo "report: ${LS_REPORT}"
exit "${LS_FAILED:-0}"
