package hello_from_class_to_top;

import java.util.logging.Logger;
import temper.core.Core;
import temper.core.Core.Console;

public final class HelloFromClassToTopGlobal {
    private HelloFromClassToTopGlobal() {}

    static final Console console;

    static void say(String message) {
        console.log(message);
    }

    static {
        console = Core.getConsole(Logger.getLogger("hello_from_class_to_top"));
        Something something = new Something("Hello, World!");
        something.sayMessage();
    }
}
