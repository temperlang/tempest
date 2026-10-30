package basics;

import java.util.logging.Logger;
import temper.core.Core;
import temper.core.Core.Console;

public final class BasicsGlobal {
    private BasicsGlobal() {}

    static {
        Console console = Core.getConsole(Logger.getLogger("basics"));
        console.log("1.min(3)");
        console.log(Integer.toString(Math.min(1, 3)));
        console.log("3.min(1)");
        console.log(Integer.toString(Math.min(3, 1)));
        console.log("1.max(3)");
        console.log(Integer.toString(Math.max(1, 3)));
        console.log("3.max(1)");
        console.log(Integer.toString(Math.max(3, 1)));
    }
}
