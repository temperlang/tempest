from asserts.asserts import (
    Vec2,
    _TestHelper,
    _assert_more_things,
    _norm_squared,
)
from temper_core import float_eq
from temper_std.testing import Test
from unittest import TestCase


class TestAsserts(TestCase):
    def test_a_test_case(self, /) -> None:
        'a test case'
        test: Test = Test()
        try:
            test.assert_(float_eq(_norm_squared(Vec2(0.0, 0.0)), 0.0), lambda: 'Oh noes')
        finally:
            test.soft_fail_to_hard()

    def test_a_failing_test(self, /) -> None:
        'a failing test'
        test: Test = Test()
        try:
            test.assert_(False, lambda: "because it's wrong")
            helper = _TestHelper(3, 2)
            _assert_more_things(test, helper, (('bye',),))
        finally:
            test.soft_fail_to_hard()

    def test_exported(self, /) -> None:
        'exported'
        test: Test = Test()
        try:
            vec = Vec2(1.5, 2.5)
            actual = vec.to_string()
            expected = '(1.5, 2.5) size squared 8.5'
            test.assert_(actual == expected, lambda: f'expected vec.toString() == ({expected}) not ({actual})')
        finally:
            test.soft_fail_to_hard()
