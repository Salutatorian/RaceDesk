import pathlib
import struct
import unittest

ROOT = pathlib.Path(__file__).parents[1]

class BrandingTests(unittest.TestCase):
    def test_windows_icon_includes_small_and_large_sizes(self):
        data = (ROOT / 'assets' / 'racedesk.ico').read_bytes()
        reserved, kind, count = struct.unpack_from('<HHH', data)
        self.assertEqual((reserved, kind), (0, 1))
        sizes = {data[6 + i * 16] or 256 for i in range(count)}
        self.assertTrue({16, 32, 48, 64, 256}.issubset(sizes))

    def test_window_icon_matches_original_tray_colours(self):
        data = (ROOT / 'assets' / 'racedesk-64.rgba').read_bytes()
        self.assertEqual(len(data), 64 * 64 * 4)
        pixels = {tuple(data[i:i + 4]) for i in range(0, len(data), 4)}
        self.assertEqual(pixels, {(110, 231, 183, 255), (21, 29, 40, 255)})

if __name__ == '__main__':
    unittest.main()
