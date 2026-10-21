import unittest
from featurization.stats import hex_string_entropy


class TestHexStringEntropy(unittest.TestCase):
    def test_uneven_input(self):
        self.assertEqual(hex_string_entropy("abcde"), hex_string_entropy("0abcde"))
        self.assertEqual(hex_string_entropy("a0a0a"), hex_string_entropy("0a0a0a"))
        self.assertEqual(hex_string_entropy("aff0a"), hex_string_entropy("0aff0a"))


if __name__ == "__main__":
    unittest.main()
