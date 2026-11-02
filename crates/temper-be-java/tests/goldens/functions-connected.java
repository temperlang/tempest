package work;

import java.util.logging.Logger;
import temper.core.Core;
import temper.core.Core.Console;
import temper.core.Nullable;

public final class WorkGlobal {
    private WorkGlobal() {}

    static int sumOf3(int a, int b, int c) {
        return a + b + c;
    }

    public static int sum(int i, int j, @Nullable Integer bonus) {
        int bonus2 = bonus != null ? bonus : 0;
        return WorkConnected.sum(i, j, bonus2);
    }

    public static int sum(int i, int j) {
        return sum(i, j, null);
    }

    public static int length(@Nullable String string) {
        return WorkConnected.length(string);
    }

    public static int length() {
        return length(null);
    }

    public static int twice(int i) {
        return WorkConnected.twice(i);
    }

    static int prod(Hidden hidden, int j) {
        return WorkConnected.prod(hidden, j);
    }

    static {
        Console console = Core.getConsole(Logger.getLogger("work"));
        console.log("sum(1, 2): " + sum(1, 2));
        console.log("length \"\" vs null vs (): " + length("") + " vs " + length(null) + " vs " + length());
        console.log("twice(5): " + twice(5));
        boolean isConnected = sum(1, 2) == 3;
        if (isConnected) {
            console.log("prod(new Hidden(1), 2): " + prod(new Hidden(1), 2));
        }
        Hider hider = new Hider(3);
        console.log("new Hider(3).plus(4): " + hider.plus(4));
        if (isConnected) {
            console.log("new Hider(3).times(4): " + hider.times(4));
        }
    }
}
