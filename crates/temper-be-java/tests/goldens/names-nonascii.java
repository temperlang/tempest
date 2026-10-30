package nonascii;

import java.util.logging.Logger;
import temper.core.Core;
import temper.core.Core.Console;

public final class NonasciiGlobal {
    private NonasciiGlobal() {}

    static final Console console;

    static void sayHello(String όνομα) {
        console.log("Hello, " + όνομα + "!");
    }

    static {
        console = Core.getConsole(Logger.getLogger("nonascii"));
        sayHello("World");
    }
}
