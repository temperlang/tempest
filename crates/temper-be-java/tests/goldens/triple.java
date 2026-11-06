package triple;

import java.util.logging.Logger;
import temper.core.Core;
import temper.core.Core.Console;

public final class TripleGlobal {
    private TripleGlobal() {}

    static int triple(int n) {
        return n * 3;
    }

    static {
        Console console = Core.getConsole(Logger.getLogger("triple"));
        console.log("triple(14) = " + triple(14));
    }
}
