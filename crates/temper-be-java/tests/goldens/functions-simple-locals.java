package simple_locals;

import java.util.function.IntUnaryOperator;
import java.util.logging.Logger;
import temper.core.Core;
import temper.core.Core.Console;

public final class SimpleLocalsGlobal {
    private SimpleLocalsGlobal() {}

    static final Console console;

    static int callIt(int i, IntUnaryOperator f) {
        return 3 * f.applyAsInt(i);
    }

    static void enclose(int i) {
        int result = callIt(2, j -> i + j);
        console.log(Integer.toString(result));
    }

    static {
        console = Core.getConsole(Logger.getLogger("simple_locals"));
        enclose(1);
    }
}
