package static_properties;

import java.util.logging.Logger;
import temper.core.Core;
import temper.core.Core.Console;

public final class StaticPropertiesGlobal {
    private StaticPropertiesGlobal() {}

    static final Console console;

    static {
        console = Core.getConsole(Logger.getLogger("static_properties"));
        console.log("Simple.simon.name is " + Simple.simon.name);
        console.log("Simple.value is " + Simple.value);
        console.log("Simple.other is " + Simple.other);
        C.sayHi();
        I.sayBye();
    }
}
