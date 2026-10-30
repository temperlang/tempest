package testing;

import java.util.logging.Logger;
import temper.core.Core;
import temper.core.Core.Console;

public final class TestingGlobal {
    private TestingGlobal() {}

    static volatile int runs;
    static final int base;

    public static int twice(int n) {
        return n * 2;
    }

    static {
        Console console = Core.getConsole(Logger.getLogger("testing"));
        runs = 0;
        base = 10;
        console.log("init " + twice(1));
    }
}
