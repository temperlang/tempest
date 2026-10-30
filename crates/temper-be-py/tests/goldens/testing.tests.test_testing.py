import testing.testing as _m_testing_testing
from temper_core import int_add
from temper_std.testing import Test
from testing.testing import _base, twice
from unittest import TestCase


class TestTesting(TestCase):
    def test_twice_works(self, /) -> None:
        'twice works'
        test: Test = Test()
        try:
            x = twice(2)
            _m_testing_testing._runs = int_add(int_add(_m_testing_testing._runs, x), _base)
        finally:
            test.soft_fail_to_hard()

    def test_names_punctuation_quotes_and_ünïcode(self, /) -> None:
        "names: punctuation, 'quotes', and ünïcode"
        t: Test = Test()
        try:
            twice(_m_testing_testing._runs)
        finally:
            t.soft_fail_to_hard()
