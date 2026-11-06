package libraries;

import java.util.List;
import java.util.function.DoubleConsumer;
import java.util.function.DoubleUnaryOperator;
import java.util.logging.Logger;
import temper.core.Core;
import temper.core.Core.Console;
import temper.core.Nullable;

public final class LibrariesGlobal {
    private LibrariesGlobal() {}

    public static Grid parseGrid(String s) {
        if (s.equals("")) {
            throw Core.bubble();
        }
        return new Grid(1, List.of(Core.stringToFloat64(s)));
    }

    static int rows(List<Double> values, int ncols) {
        try {
            return Core.divIntInt(values.size(), ncols);
        } catch (RuntimeException ignored) {
            throw new AssertionError("panic");
        }
    }

    static Grid scaled(Grid grid) {
        if (grid.ncols == 0) {
            throw Core.bubble();
        }
        return grid;
    }

    static List<Double> cubes(Grid grid) {
        try {
            List<Double> more = Core.listMapDoubleToDouble(scaled(grid).values, x -> x * x * x);
            return more;
        } catch (RuntimeException ignored) {
            throw new AssertionError("panic");
        }
    }

    static void each(List<Double> values, DoubleConsumer f) {
        {
            int i = 0;
            while (i < values.size()) {
                f.accept(Core.listGet(values, i));
                i = i + 1;
            }
        }
    }

    static double total(List<Double> values) {
        class SumCell {
            double value;
        }
        final SumCell sum = new SumCell();
        sum.value = 0.0;
        DoubleConsumer fn = x -> {
            try {
                double part = x / 2.0;
                sum.value = sum.value + part;
            } catch (RuntimeException ignored) {
                throw new AssertionError("panic");
            }
        };
        each(values, fn);
        return sum.value;
    }

    static double half(double x) {
        try {
            return x / 2.0;
        } catch (RuntimeException ignored) {
            throw new AssertionError("panic");
        }
    }

    static double cube(double x) {
        return x * 0.25;
    }

    static double corner(Grid grid) {
        try {
            return grid.get(1, 1);
        } catch (RuntimeException ignored) {
            return -1.0;
        }
    }

    static @Nullable Grid maybe(int n) {
        if (n > 0) {
            return new Grid(n, List.of(1.0));
        } else {
            return null;
        }
    }

    static List<Double> mapped(Grid grid) {
        Grid t;
        try {
            t = scaled(grid);
        } catch (RuntimeException ignored) {
            throw new AssertionError("panic");
        }
        DoubleUnaryOperator fn = x -> {
            if (Double.compare(x, 1.5) > 0) {
                return x * 2.0;
            } else {
                return x;
            }
        };
        return Core.listMapDoubleToDouble(t.values, fn);
    }

    static @Nullable String text(int n) {
        if (n > 0) {
            return "x";
        } else {
            return null;
        }
    }

    static @Nullable Integer count(int n) {
        if (n > 0) {
            return n;
        } else {
            return null;
        }
    }

    static @Nullable Casing casing(int n) {
        if (n > 0) {
            return Casing.first;
        } else {
            return null;
        }
    }

    static String captured(String s) {
        @Nullable Grid found_;
        try {
            found_ = parseGrid(s);
        } catch (RuntimeException ignored) {
            found_ = null;
        }
        @Nullable Grid found = found_;
        return Core.listJoinBool(Core.listMapIntToBool(List.of(1, 2), x -> found == null), ",", b -> Boolean.toString(b));
    }

    static String bidi() {
        return "look\u202eout";
    }

    static String word(int n) {
        if (n > 1) {
            return "b";
        } else {
            return "a";
        }
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
        Console console = Core.getConsole(Logger.getLogger("libraries"));
        Grid grid = new Grid(2, List.of(1.0, 2.0, 3.0, 4.0));
        console.log(rows(grid.values, 2) + " " + Core.float64ToString(half(3.0)) + " " + Core.float64ToString(cube(8.0)));
        console.log(Core.float64ToString(corner(grid)));
        console.log(Core.float64ToString(total(grid.values)));
        @Nullable String none = text(0);
        @Nullable Integer some = count(3);
        @Nullable String part = none;
        String t = (part == null ? "null" : part) + " ";
        @Nullable Integer part2 = some;
        String t2 = t + (part2 == null ? "null" : Integer.toString(part2)) + " ";
        @Nullable Grid subject = maybe(0);
        @Nullable Integer part3 = subject == null ? null : Integer.valueOf(subject.ncols);
        console.log(t2 + (part3 == null ? "null" : Integer.toString(part3)));
        console.log((maybe(1) != null) + " " + (maybe(0) != null) + " " + (Grid.parse("") == null));
        console.log(Casing.first.toString());
        console.log(Core.listJoinDouble(mapped(grid), ",", x -> Core.float64ToString(x)));
        console.log(Core.listJoinDouble(cubes(grid), ",", x -> Core.float64ToString(x)));
        @Nullable Casing left = casing(1);
        String t3 = (left == null ? false : left.eq(Casing.first)) + " ";
        @Nullable Casing left2 = casing(0);
        String t4 = t3 + (left2 == null ? false : left2.eq(Casing.first)) + " ";
        @Nullable Casing left3 = casing(0);
        @Nullable Casing right = casing(0);
        console.log(t4 + !(left3 == null ? right == null : right == null ? false : left3.eq(right)));
        console.log(captured("") + " " + captured("2.5"));
        console.log(Boolean.toString(!bidi().equals("lookout")));
        console.log(compareCodePoints(word(1), word(2)) + " " + compareCodePoints(word(2), word(1)) + " " + compareCodePoints(word(2), word(3)));
        String named = word(2);
        console.log(Boolean.toString(named.equals(word(3)) && !(named + "!").equals(named)));
    }
}
