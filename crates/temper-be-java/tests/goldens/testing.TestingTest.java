package testing;

import temper.std.testing.Test;

public final class TestingTest {
    @org.junit.jupiter.api.Test
    public void twiceWorks() {
        Test test = new Test();
        try {
            int x = TestingGlobal.twice(2);
            TestingGlobal.runs = TestingGlobal.runs + x + TestingGlobal.base;
        } finally {
            test.softFailToHard();
        }
    }

    @org.junit.jupiter.api.Test
    public void namesPunctuationQuotesAndÜnïcode() {
        Test t = new Test();
        try {
            TestingGlobal.twice(TestingGlobal.runs);
        } finally {
            t.softFailToHard();
        }
    }
}
