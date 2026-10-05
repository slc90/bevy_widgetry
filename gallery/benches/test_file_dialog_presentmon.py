import io
import unittest

from file_dialog_presentmon import correlate, load_rows


class PresentationContract(unittest.TestCase):
    def setUp(self):
        self.protocol = {
            "clock_id": "fixture", "process_id": 42,
            "qpc_calibration": {"qpc": 1000000000, "frequency": 1000000000,
                                "before_ns": 0, "after_ns": 200},
        }
        self.content = {
            "sample": 1, "root": 11, "session": "session", "native_window": 12,
            "camera": 13, "app_frame": 5, "render_start_ns": 1000000,
            "ns": 2000000, "window_count": 2, "hwnd": 123,
        }
        self.row = {"ProcessID": "42", "SwapChainAddress": "0x02",
                    "QPCTime": "1001500000", "Dropped": "0", "msUntilDisplayed": "2.5"}

    def test_exact_render_interval_maps_display_timestamp(self):
        result = correlate(self.protocol, self.content, [self.row], "0x01", "present.csv")
        self.assertEqual(result["ns"], 4000100)
        self.assertEqual(result["uncertainty_ns"], 101)
        self.assertEqual(result["app_frame"], 5)

    def test_primary_surface_cannot_be_dialog_evidence(self):
        row = dict(self.row, SwapChainAddress="0x01")
        with self.assertRaisesRegex(ValueError, "exactly one"):
            correlate(self.protocol, self.content, [row], "0x01", "present.csv")

    def test_dropped_frame_is_not_zero_display_latency(self):
        row = dict(self.row, Dropped="1", msUntilDisplayed="0")
        with self.assertRaisesRegex(ValueError, "not displayed"):
            correlate(self.protocol, self.content, [row], "0x01", "present.csv")

    def test_missing_display_latency_is_not_zero(self):
        row = dict(self.row, msUntilDisplayed="NA")
        with self.assertRaisesRegex(ValueError, "not displayed"):
            correlate(self.protocol, self.content, [row], "0x01", "present.csv")

    def test_ambiguous_surface_mapping_is_rejected(self):
        with self.assertRaisesRegex(ValueError, "exactly one"):
            correlate(self.protocol, self.content, [self.row, dict(self.row, SwapChainAddress="0x03")],
                      "0x01", "present.csv")

    def test_extra_native_window_invalidates_exclusive_mapping(self):
        with self.assertRaisesRegex(ValueError, "two windows"):
            correlate(self.protocol, dict(self.content, window_count=3), [self.row], "0x01", "present.csv")

    def test_other_process_or_other_frame_is_rejected(self):
        for row in [dict(self.row, ProcessID="43"), dict(self.row, QPCTime="1002500000")]:
            with self.assertRaisesRegex(ValueError, "exactly one"):
                correlate(self.protocol, self.content, [row], "0x01", "present.csv")

    def test_schema_drift_is_an_error(self):
        with self.assertRaisesRegex(ValueError, "schema"):
            load_rows(io.StringIO("CPUStartTime,DisplayedTime\n1,2\n"))

    def test_bad_clock_calibration_is_rejected(self):
        for calibration in [None, {"qpc": 1, "frequency": 0, "before_ns": 0, "after_ns": 1}]:
            with self.assertRaises(ValueError):
                correlate(dict(self.protocol, qpc_calibration=calibration), self.content,
                          [self.row], "0x01", "present.csv")

    def test_clock_drift_invalidates_mapping(self):
        protocol = dict(self.protocol, clock_check={"qpc": 2000000000, "frequency": 1000000000,
                                                  "before_ns": 1001000000, "after_ns": 1001000200})
        with self.assertRaisesRegex(ValueError, "drift"):
            correlate(protocol, self.content, [self.row], "0x01", "present.csv")


if __name__ == "__main__":
    unittest.main()
