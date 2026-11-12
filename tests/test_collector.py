import importlib.util
import pathlib
import unittest

spec = importlib.util.spec_from_file_location('collect', pathlib.Path(__file__).parents[1] / 'collector' / 'collect.py')
collect = importlib.util.module_from_spec(spec)
spec.loader.exec_module(collect)

class CollectorTests(unittest.TestCase):
    def test_extracts_literal_without_executing_javascript(self):
        source = '''x=JSON.parse('[{"seriesid":1,"seriesname":"Driver\\\'s cup","tracks":[]}]'); throw Error('never execute')'''
        self.assertEqual(collect.embedded_arrays(source)[0][0]['seriesname'], "Driver's cup")

    def test_rejects_missing_catalog(self):
        with self.assertRaises(ValueError):
            collect.extract('x=JSON.parse(\'[]\')')

    def test_rejects_nonliteral(self):
        self.assertEqual(collect.embedded_arrays('JSON.parse(stealCookies())'), [])

if __name__ == '__main__':
    unittest.main()
