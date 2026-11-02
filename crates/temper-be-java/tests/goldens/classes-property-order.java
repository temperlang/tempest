package property_order;

import java.util.logging.Logger;
import temper.core.Core;
import temper.core.Core.Console;

public final class PropertyOrderGlobal {
    private PropertyOrderGlobal() {}

    static final Console console;

    static {
        console = Core.getConsole(Logger.getLogger("property_order"));
        console.log("about to create instance");
        C c = new C();
        console.log("about to get backed");
        console.log("c.backed=" + c.backed);
        console.log("about to get computed");
        console.log("c.computed=" + c.getComputed());
        console.log("done");
    }
}
