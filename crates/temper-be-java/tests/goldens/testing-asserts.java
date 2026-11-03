package asserts;

import java.util.List;
import temper.core.Core;
import temper.std.testing.Test;

public final class AssertsGlobal {
    private AssertsGlobal() {}

    static double normSquared(Vec2 vec) {
        return vec.x * vec.x + vec.y * vec.y;
    }

    static void assertMoreThings(Test t, TestHelper helper, List<List<String>> strings) {
        {
            int i = 0;
            while (i < strings.size()) {
                {
                    int j = 0;
                    while (j < Core.listGet(strings, i).size()) {
                        {
                            int actual = helper.i;
                            int expected = helper.j;
                            t.assert_(actual == expected, () -> "expected helper.i == (" + expected + ") not (" + actual + ")");
                        }
                        String s = Core.listGet(Core.listGet(strings, i), j);
                        {
                            String actual = s;
                            String expected = "hi";
                            t.assert_(actual.equals(expected), () -> "expected s == (" + expected + ") not (" + actual + ")");
                        }
                        j = j + 1;
                    }
                }
                i = i + 1;
            }
        }
    }

    static {
        String name = "case";
    }
}
