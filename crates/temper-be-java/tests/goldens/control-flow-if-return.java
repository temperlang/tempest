package if_return;

import java.util.logging.Logger;
import temper.core.Core;
import temper.core.Core.Console;

public final class IfReturnGlobal {
    private IfReturnGlobal() {}

    static String explicit(boolean b) {
        if (b) {
            return "true";
        }
        return "false";
    }

    static String halfAndHalf(boolean b) {
        if (b) {
            return "true";
        }
        return "false";
    }

    static String implicit(boolean b) {
        if (b) {
            return "true";
        } else {
            return "false";
        }
    }

    static String nested(boolean a, boolean b) {
        String result = "neither";
        if (a) {
            if (b) {
                result = "both";
            }
        } else {
            result = "not a";
        }
        return result;
    }

    static {
        Console console = Core.getConsole(Logger.getLogger("if_return"));
        boolean t = true;
        t = t;
        boolean f = !t;
        console.log("explicit    true  -> " + explicit(t));
        console.log("explicit    false -> " + explicit(f));
        console.log("halfAndHalf true  -> " + halfAndHalf(t));
        console.log("halfAndHalf false -> " + halfAndHalf(f));
        console.log("implicit    true  -> " + implicit(t));
        console.log("implicit    false -> " + implicit(f));
        console.log("nested      true  false -> " + nested(t, f));
        console.log("nested      false true  -> " + nested(f, t));
    }
}
