#!/usr/bin/env python3
"""Foreground protocol checks without launching apps or requiring desktop permissions."""
import importlib.util
from pathlib import Path
import subprocess
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('capture', Path(__file__).with_name('gpu-frame-capture.py'))
capture = importlib.util.module_from_spec(spec)
spec.loader.exec_module(capture)


class ForegroundProtocolTests(unittest.TestCase):
    def test_preserves_target_metadata(self):
        output = '{"frontmost":true,"application_active":true,"frontmost_pid":42,"window_width":1920,"window_height":1080}'
        with patch.object(capture.subprocess, 'run', return_value=subprocess.CompletedProcess([], 0, output, '')):
            result = capture.foreground(42, Path('/helper'))
        self.assertTrue(result['frontmost'])
        self.assertEqual(result['window_width'], 1920)
        self.assertEqual(result['helper_stdout'], output)
        self.assertEqual(result['helper_returncode'], 0)

    def test_failure_preserves_stderr_and_cannot_claim_foreground(self):
        with patch.object(capture.subprocess, 'run', return_value=subprocess.CompletedProcess([], 2, '{}', 'actual failure')):
            result = capture.foreground(42, Path('/helper'))
        self.assertFalse(result['frontmost'])
        self.assertEqual(result['helper_stderr'], 'actual failure')
        self.assertEqual(result['helper_returncode'], 2)

    def test_wrong_frontmost_pid_is_rejected(self):
        output = '{"frontmost":true,"application_active":true,"frontmost_pid":24}'
        with patch.object(capture.subprocess, 'run', return_value=subprocess.CompletedProcess([], 0, output, '')):
            self.assertFalse(capture.foreground(42, Path('/helper'))['frontmost'])

    def test_timeout_keeps_byte_output_json_serializable(self):
        error = subprocess.TimeoutExpired(['/helper'], 5, output=b'partial', stderr=b'slow helper')
        with patch.object(capture.subprocess, 'run', side_effect=error):
            result = capture.foreground(42, Path('/helper'))
        self.assertFalse(result['frontmost'])
        self.assertEqual(result['helper_stdout'], 'partial')
        self.assertEqual(result['helper_stderr'], 'slow helper')
        capture.json.dumps(result)


if __name__ == '__main__':
    unittest.main()
