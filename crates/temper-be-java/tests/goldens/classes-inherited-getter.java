package inherited_getter;

import java.util.logging.Logger;
import temper.core.Core;
import temper.core.Core.Console;

public final class InheritedGetterGlobal {
    private InheritedGetterGlobal() {}

    static {
        Console console = Core.getConsole(Logger.getLogger("inherited_getter"));
        C c = new C();
        console.log("Goodbye, gotta run, " + c.getPlace() + "!");
    }
}
