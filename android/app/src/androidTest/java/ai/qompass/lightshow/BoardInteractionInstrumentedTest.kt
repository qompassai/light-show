package ai.qompass.lightshow

import android.content.Intent
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import androidx.test.uiautomator.By
import androidx.test.uiautomator.UiDevice
import androidx.test.uiautomator.Until
import org.junit.After
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith

/**
 * Black-box instrumentation tests for the puzzle board's drag-to-route and
 * pill-selection interactions.
 *
 * Light Show renders entirely inside a single `android.app.NativeActivity`
 * GL surface, so there is no Espresso-visible view hierarchy to assert
 * against. Instead these tests drive real touch input via UiAutomator at
 * screen coordinates derived (at runtime, from the device's actual display
 * size) from the same world-space geometry `game/src/board.rs`'s
 * `grid_to_world`/`pill_world_pos` use for the bundled "First Light" level
 * (`game/assets/levels/world1_level1.json`, which `CurrentLevelIndex`
 * defaults to), then assert on structured Logcat events. Those events are
 * only emitted when the native library is built with the
 * `instrumented-test-logging` Cargo feature (see `test_log!` in
 * `game/src/lib.rs`, and its call sites in
 * `board::handle_pointer_input`/`states::playing::setup_level`) — see
 * `docs/BUILD.md` for how to build and run this locally and in CI.
 */
@RunWith(AndroidJUnit4::class)
class BoardInteractionInstrumentedTest {

    private lateinit var device: UiDevice

    companion object {
        private const val TARGET_PACKAGE = "ai.qompass.lightshow"
        private const val LOGCAT_TAG = "LightShow"
        private const val LAUNCH_TIMEOUT_MS = 8_000L
        private const val LOG_WAIT_TIMEOUT_MS = 10_000L
        private const val LOG_POLL_INTERVAL_MS = 200L
        private const val ACTION_SETTLE_MS = 500L
        private const val SWIPE_STEPS = 24

        // World-space coordinates mirrored from game/src/board.rs's
        // grid_to_world() applied to game/assets/levels/world1_level1.json
        // ("First Light"): node 1 ("Splice Enclosure 14+00") sits at grid
        // (1, 0) -> world (0, 300); node 2 ("ONT") sits at grid (2, 0) ->
        // world (200, 300). That level's only multi-choice pair is
        // (1, 2) -- fusion splice (slot 0) vs. mechanical splice (slot 1)
        // -- and pill_world_pos() centers the two pills 70 world units
        // apart, straddling the node1-node2 edge's midpoint.
        private const val NODE_1_WORLD_X = 0f
        private const val NODE_1_WORLD_Y = 300f
        private const val NODE_2_WORLD_X = 200f
        private const val NODE_2_WORLD_Y = 300f
        private const val MECHANICAL_PILL_WORLD_X = 100f
        private const val MECHANICAL_PILL_WORLD_Y = 335f
    }

    @Before
    fun setUp() {
        device = UiDevice.getInstance(InstrumentationRegistry.getInstrumentation())
        // Force-stop first in case a previous run left the process alive:
        // Bevy's game state (and therefore whether it re-emits
        // "level_ready") persists across activity resume within the same
        // process, so each test needs a genuinely cold start.
        device.executeShellCommand("am force-stop $TARGET_PACKAGE")
        device.executeShellCommand("logcat -c")
        launchApp()
        assertTrue(
            "app never logged 'level_ready' after launch -- was it built with " +
                "the instrumented-test-logging Cargo feature? See docs/BUILD.md",
            waitForLogLine("level_ready"),
        )
    }

    @After
    fun tearDown() {
        device.executeShellCommand("am force-stop $TARGET_PACKAGE")
    }

    @Test
    fun dragFromNodeOneToNodeTwo_connectsDefaultFusionChoice() {
        val (x1, y1) = worldToScreen(NODE_1_WORLD_X, NODE_1_WORLD_Y)
        val (x2, y2) = worldToScreen(NODE_2_WORLD_X, NODE_2_WORLD_Y)

        assertTrue("swipe gesture from node 1 to node 2 failed", device.swipe(x1, y1, x2, y2, SWIPE_STEPS))
        Thread.sleep(ACTION_SETTLE_MS)

        assertTrue(
            "expected a 'connect from=1 to=2' Logcat event after dragging node 1 -> node 2",
            waitForLogLine("connect from=1 to=2"),
        )
    }

    @Test
    fun tappingMechanicalPill_selectsItDirectlyWithoutRequiringADrag() {
        val (x, y) = worldToScreen(MECHANICAL_PILL_WORLD_X, MECHANICAL_PILL_WORLD_Y)

        assertTrue("tap on the mechanical splice pill failed", device.click(x, y))
        Thread.sleep(ACTION_SETTLE_MS)

        assertTrue(
            "expected a 'select from=1 to=2 slot=1' Logcat event after tapping the mechanical pill",
            waitForLogLine("select from=1 to=2 slot=1"),
        )
    }

    private fun launchApp() {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val intent = context.packageManager.getLaunchIntentForPackage(TARGET_PACKAGE)
            ?: error("No launch intent found for package $TARGET_PACKAGE -- is it installed?")
        intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TASK)
        context.startActivity(intent)
        device.wait(Until.hasObject(By.pkg(TARGET_PACKAGE)), LAUNCH_TIMEOUT_MS)
    }

    /**
     * Converts a board world-space point to device screen pixels, mirroring
     * the default Bevy 2D camera board.rs relies on: origin at the
     * viewport center, world Y increasing upward vs. screen Y increasing
     * downward. Deriving this from the device's *actual* runtime display
     * size (rather than hard-coding pixel values for one assumed
     * resolution) keeps the test correct across whatever emulator/device
     * profile actually runs it.
     */
    private fun worldToScreen(worldX: Float, worldY: Float): Pair<Int, Int> {
        val screenX = device.displayWidth / 2 + worldX
        val screenY = device.displayHeight / 2 - worldY
        return Pair(screenX.toInt(), screenY.toInt())
    }

    /** Polls `logcat -d` (filtered to our tag) until `needle` appears or times out. */
    private fun waitForLogLine(needle: String): Boolean {
        val deadline = System.currentTimeMillis() + LOG_WAIT_TIMEOUT_MS
        while (System.currentTimeMillis() < deadline) {
            val log = device.executeShellCommand("logcat -d -s $LOGCAT_TAG:I")
            if (log.contains(needle)) {
                return true
            }
            Thread.sleep(LOG_POLL_INTERVAL_MS)
        }
        return false
    }
}
