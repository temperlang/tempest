package loop_reenterable;

import java.util.logging.Logger;
import temper.core.Core;
import temper.core.Core.Console;

public final class LoopReenterableGlobal {
    private LoopReenterableGlobal() {}

    static {
        Console console = Core.getConsole(Logger.getLogger("loop_reenterable"));
        int i = 0;
        while (i < 2) {
            int x = i;
            console.log(Integer.toString(x));
            i = i + 1;
        }
    }
}
