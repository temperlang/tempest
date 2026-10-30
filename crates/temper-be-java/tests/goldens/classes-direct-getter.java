package direct_getter;

import java.util.logging.Logger;
import temper.core.Core;
import temper.core.Core.Console;

public final class DirectGetterGlobal {
    private DirectGetterGlobal() {}

    static {
        Console console = Core.getConsole(Logger.getLogger("direct_getter"));
        C c = new C();
        console.log("Hello, " + c.getPlace() + "!");
    }
}
