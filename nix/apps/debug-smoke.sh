# ls-debug-smoke: standalone lldb-dap breakpoint smoke test.
#
# Drives lldb-dap over DAP against the osp_sim unit-test binary:
# sets a breakpoint on osp_sim::ReceiveWindow::margin (source-line
# fallback), launches the single test that calls it, and confirms the
# breakpoint is hit with the function on the stack.
# Verdict is PASS / FAIL / SKIP - never faked.

ls_check_deps cargo python3 lldb-dap git
ls_start_report "debug-smoke"

ls_debug_smoke
ls_verify_clean

echo "report: ${LS_REPORT}"
if [ "${LS_FAILED}" -ne 0 ]; then
    echo "DEBUG-SMOKE FAILED - see ${LS_REPORT}" >&2
fi
exit "${LS_FAILED}"
