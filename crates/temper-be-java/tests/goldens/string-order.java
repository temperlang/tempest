package string_order;

import java.util.logging.Logger;
import temper.core.Core;
import temper.core.Core.Console;

public final class StringOrderGlobal {
    private StringOrderGlobal() {}

    static String show(String a, String b) {
        return compareCodePoints(a, b) + " " + (compareCodePoints(a, b) < 0) + " " + (compareCodePoints(a, b) <= 0) + " " + (compareCodePoints(a, b) > 0) + " " + (compareCodePoints(a, b) >= 0);
    }

    static int cmp(double a, double b) {
        return Double.compare(a, b);
    }

    /** Compares strings by code point, as Temper does: -1, 0, or 1. */
    private static int compareCodePoints(String a, String b) {
        int length = Math.min(a.length(), b.length());
        for (int i = 0; i < length; i++) {
            char x = a.charAt(i);
            char y = b.charAt(i);
            if (x != y) {
                boolean surrogate = Character.isSurrogate(x);
                if (surrogate != Character.isSurrogate(y)) {
                    return surrogate ? 1 : -1;
                }
                return x < y ? -1 : 1;
            }
        }
        return Integer.signum(a.length() - b.length());
    }

    static {
        Console console = Core.getConsole(Logger.getLogger("string_order"));
        console.log(show("｡", "😀"));
        console.log(show("😀", "｡"));
        console.log(show("", "𐀀"));
        console.log(show("￿", "􏿿"));
        console.log(show("a｡b", "a😀"));
        console.log(show("😀", "😁"));
        console.log(show("😀", "😀"));
        console.log(show("😀", "😀a"));
        console.log(show("", "😀"));
        console.log(show("", ""));
        console.log(show("z", "a"));
        console.log(show("abc", "abd"));
        console.log(cmp(2.5, 1.0) + " " + cmp(1.0, 2.5) + " " + cmp(1.5, 1.5));
    }
}
