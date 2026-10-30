package module_state;

import java.util.logging.Logger;
import temper.core.Core;
import temper.core.Core.Console;

public final class ModuleStateGlobal {
    private ModuleStateGlobal() {}

    public static final String greeting;
    static volatile int count;
    static final int limit;
    public static volatile int total;
    public static volatile int ticks;

    static int bump() {
        synchronized (ModuleStateGlobal.class) {
            count = count + 1;
        }
        total = total + count;
        return count;
    }

    public static void tick() {
        synchronized (ModuleStateGlobal.class) {
            ticks = ticks + 1;
        }
    }

    public static String report() {
        return greeting + " " + bump() + " of " + limit + " " + total;
    }

    static {
        Console console = Core.getConsole(Logger.getLogger("module_state"));
        greeting = "hi";
        count = 0;
        limit = 3;
        total = 10;
        ticks = 0;
        int local = 1;
        local = local + limit;
        console.log(report());
        console.log(report());
        console.log("local " + local);
    }
}
