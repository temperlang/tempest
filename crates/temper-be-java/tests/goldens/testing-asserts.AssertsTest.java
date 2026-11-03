package asserts;

import java.util.List;
import temper.std.testing.Test;

public final class AssertsTest {
    @org.junit.jupiter.api.Test
    public void aTestCase() {
        Test test = new Test();
        try {
            test.assert_(Double.compare(AssertsGlobal.normSquared(new Vec2(0.0, 0.0)), 0.0) == 0, () -> "Oh noes");
        } finally {
            test.softFailToHard();
        }
    }

    @org.junit.jupiter.api.Test
    public void aFailingTest() {
        Test test = new Test();
        try {
            test.assert_(false, () -> "because it's wrong");
            TestHelper helper = new TestHelper(3, 2);
            AssertsGlobal.assertMoreThings(test, helper, List.of(List.of("bye")));
        } finally {
            test.softFailToHard();
        }
    }

    @org.junit.jupiter.api.Test
    public void exported() {
        Test test = new Test();
        try {
            Vec2 vec = new Vec2(1.5, 2.5);
            {
                String actual = vec.toString();
                String expected = "(1.5, 2.5) size squared 8.5";
                test.assert_(actual.equals(expected), () -> "expected vec.toString() == (" + expected + ") not (" + actual + ")");
            }
        } finally {
            test.softFailToHard();
        }
    }
}
