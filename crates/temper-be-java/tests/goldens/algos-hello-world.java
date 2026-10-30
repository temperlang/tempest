package hello_world;

import java.util.logging.Logger;
import temper.core.Core;
import temper.core.Core.Console;

public final class HelloWorldGlobal {
    private HelloWorldGlobal() {}

    static {
        Console console = Core.getConsole(Logger.getLogger("hello_world"));
        console.log("Hello, World!");
    }
}
