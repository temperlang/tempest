package as_values;

import java.util.function.Consumer;
import java.util.logging.Logger;
import temper.core.Core;
import temper.core.Core.Console;

public final class AsValuesGlobal {
    private AsValuesGlobal() {}

    static final Console console;

    static void hello(String how) {
        console.log("Called " + how + "!");
    }

    static void callIt(Consumer<String> f) {
        f.accept("by a function");
    }

    static boolean isOdd(int i) {
        if (i == 0) {
            return false;
        } else {
            return isEven(i - 1);
        }
    }

    static boolean isEven(int i) {
        if (i == 0) {
            return true;
        } else {
            return isOdd(i - 1);
        }
    }

    static {
        console = Core.getConsole(Logger.getLogger("as_values"));
        hello("directly");
        callIt(AsValuesGlobal::hello);
        new DelayedFunction(AsValuesGlobal::hello).callIt();
        console.log("Is 5 even? " + isEven(5));
        console.log("Is 6 even? " + isEven(6));
    }
}
