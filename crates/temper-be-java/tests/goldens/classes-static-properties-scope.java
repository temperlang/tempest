package static_properties_scope;

import java.util.logging.Logger;
import temper.core.Core;
import temper.core.Core.Console;

public final class StaticPropertiesScopeGlobal {
    private StaticPropertiesScopeGlobal() {}

    static String readSimpleValue() {
        return Simple.value;
    }

    static String readSimpleOther() {
        return Simple.other;
    }

    static {
        Console console = Core.getConsole(Logger.getLogger("static_properties_scope"));
        console.log("Simple.value is " + readSimpleValue());
        console.log("Simple.other is " + readSimpleOther());
    }
}
