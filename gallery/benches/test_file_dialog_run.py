import contextlib
import io
import json
from pathlib import Path
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch

from file_dialog import analyze, read_report
from file_dialog_run import main, parse_args, summarize, wait_launcher


def report(*verdicts):
    verified = bool(verdicts) and all(value is not None for value in verdicts)
    return {
        "display_status": "verified" if verified else "incomplete",
        "150ms_target_passed": all(verdicts) if verified else None,
        "samples": [{"sample": i + 1, "valid_submission": True,
                     "display_verified": value is not None,
                     "150ms_target_passed": value} for i, value in enumerate(verdicts)],
    }


def run_record(*verdicts):
    return {"run": 1, "collection_completed": True, "resources_passed": True,
            "analysis": report(*verdicts)}


class RunnerContract(unittest.TestCase):
    def test_launcher_waits_for_page_creation_and_nonzero_layout(self):
        zero = {"components": {"bevy_ui::ui_transform::UiGlobalTransform": [1, 0, 0, 1, 20, 30],
                "bevy_ui::ui_node::ComputedNode": {"size": [0, 0], "inverse_scale_factor": 1}}}
        ready = {"components": {"bevy_ui::ui_transform::UiGlobalTransform": [1, 0, 0, 1, 20, 30],
                 "bevy_ui::ui_node::ComputedNode": {"size": [80, 40], "inverse_scale_factor": 1}}}
        with patch("file_dialog_run.named_geometry", side_effect=[ValueError("not mounted"), [zero], [ready]]), \
             patch("file_dialog_run.time.sleep"):
            self.assertEqual(wait_launcher(15702, "launcher"), [20, 30])

    def test_launcher_wait_has_a_bounded_timeout(self):
        with patch("file_dialog_run.named_geometry", side_effect=ValueError("not mounted")), \
             patch("file_dialog_run.time.monotonic", side_effect=[0, 31]):
            with self.assertRaises(TimeoutError):
                wait_launcher(15702, "launcher")

    def test_strict_verdicts_and_exit_codes(self):
        for verdicts, status, code in [
            ([True, True], "passed", 0),
            ([True, False], "failed", 3),
            ([True, None], "incomplete", 4),
            ([False, None], "failed_and_incomplete", 5),
        ]:
            with self.subTest(verdicts=verdicts):
                summary = summarize([run_record(*verdicts)], 1, 2, True)
                self.assertEqual(summary["status"], status)
                self.assertEqual(summary["exit_code"], code)
                self.assertEqual(len(summary["budget_failures"]), verdicts.count(False))
                self.assertEqual(len(summary["missing_display_evidence"]), verdicts.count(None))

    def test_empty_and_missing_runs_or_samples_are_incomplete(self):
        for runs, count, samples in [([], 1, 1), ([run_record()], 1, 1),
                                    ([run_record(True)], 1, 2), ([run_record(True)], 2, 1)]:
            summary = summarize(runs, count, samples, True)
            self.assertEqual(summary["status"], "incomplete")
            self.assertFalse(summary["collection_completed"])
            self.assertFalse(summary["formal_passed"])

    def test_collection_only_never_claims_formal_acceptance(self):
        for verdict in [True, False, None]:
            summary = summarize([run_record(verdict)], 1, 1, False)
            self.assertEqual(summary["status"], "collection_only")
            self.assertEqual(summary["exit_code"], 0)
            self.assertIsNone(summary["formal_passed"])

    def test_errors_preserve_existing_failure_and_missing_evidence(self):
        summary = summarize([run_record(False, None)], 2, 2, True, ["broken report"])
        self.assertEqual(summary["status"], "error")
        self.assertEqual(summary["exit_code"], 1)
        self.assertEqual(len(summary["budget_failures"]), 1)
        self.assertEqual(len(summary["missing_display_evidence"]), 1)
        self.assertEqual(summary["errors"], ["broken report"])

    def test_multiple_runs_preserve_failed_and_incomplete_identities(self):
        runs = [run_record(False), dict(run_record(None), run=2)]
        summary = summarize(runs, 2, 1, True)
        self.assertEqual(summary["status"], "failed_and_incomplete")
        self.assertEqual(summary["budget_failures"], [{"run": 1, "sample": 1}])
        self.assertEqual(summary["missing_display_evidence"], [{"run": 2, "sample": 1}])

    def test_resource_failure_prevents_acceptance_independently_of_budget(self):
        summary = summarize([dict(run_record(True), resources_passed=False)], 1, 1, True)
        self.assertFalse(summary["formal_passed"])
        self.assertTrue(summary["budget_passed"])
        self.assertFalse(summary["resources_passed"])
        self.assertNotEqual(summary["exit_code"], 0)

    def test_runner_writes_error_summary_and_keeps_prior_results(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "new"
            args = SimpleNamespace(output=output, runs=2, samples=2, strict_display_budget=True)

            def collect_then_fail(args, reports):
                reports.append(run_record(False, None))
                raise ValueError("broken report")

            with patch("file_dialog_run.parse_args", return_value=args), \
                 patch("file_dialog_run.run_processes", side_effect=collect_then_fail), \
                 contextlib.redirect_stdout(io.StringIO()):
                self.assertEqual(main([]), 1)
            summary = json.loads((output / "summary.json").read_text(encoding="utf-8"))
            self.assertEqual(summary["status"], "error")
            self.assertEqual(summary["errors"], ["ValueError: broken report"])
            self.assertEqual(len(summary["budget_failures"]), 1)
            self.assertEqual(len(summary["missing_display_evidence"]), 1)
            self.assertEqual(len(json.loads((output / "runs.json").read_text(encoding="utf-8"))), 1)

    def test_control_conflicts_with_strict_display_before_launch(self):
        with contextlib.redirect_stderr(io.StringIO()):
            with self.assertRaises(SystemExit) as error:
                parse_args(["--output", "unused", "--fixture", "unused", "--presentmon", "unused",
                            "--control", "--strict-display-budget"])
        self.assertEqual(error.exception.code, 2)

    def test_corrupt_report_is_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "report.json"
            for value in ["{", "null", "{}", json.dumps(dict(report(True), display_status="incomplete")),
                          json.dumps(dict(report(True), samples=[{}]))]:
                path.write_text(value, encoding="utf-8")
                with self.assertRaises(ValueError):
                    read_report(path)
            path.write_text(json.dumps(report(False, None)), encoding="utf-8")
            self.assertEqual(read_report(path), report(False, None))

    def test_analyzer_keeps_150ms_upper_bound_and_uncertainty(self):
        with tempfile.TemporaryDirectory() as directory:
            directory = Path(directory)
            identity = {"sample": 1, "root": 11, "session": "session", "native_window": 12,
                        "camera": 13, "app_frame": 5}
            trace = [{"event": "protocol", "protocol": 2, "clock_id": "fixture"},
                     {"event": "activate", "sample": 1, "t_input": 0, "t_activate": 1, "operation": "OpenFile", "app_frame": 1},
                     dict(identity, event="scene", t_scene=2),
                     dict(identity, event="camera_ready", ns=3),
                     dict(identity, event="cpu_content_ready", ns=4),
                     dict(identity, event="first_content_frame", ns=5)]
            events = directory / "events.jsonl"
            events.write_text("".join(json.dumps(event) + "\n" for event in trace), encoding="utf-8")
            artifact = directory / "present.csv"
            artifact.touch()
            presentation = directory / "presentation.jsonl"
            output = directory / "report.json"
            for uncertainty, expected in [(0, True), (1, False)]:
                evidence = dict(identity, clock_id="fixture", endpoint="displayed", source="fixture",
                                artifact=str(artifact), ns=150_000_000, uncertainty_ns=uncertainty)
                presentation.write_text(json.dumps(evidence) + "\n", encoding="utf-8")
                with contextlib.redirect_stdout(io.StringIO()):
                    analyze(SimpleNamespace(events=events, presentation=presentation, output=output))
                result = read_report(output)
                self.assertIs(result["150ms_target_passed"], expected)
                self.assertIs(result["samples"][0]["150ms_target_passed"], expected)


if __name__ == "__main__":
    unittest.main()
