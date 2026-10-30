package hello_world_object;

import java.util.logging.Logger;
import temper.core.Core;
import temper.core.Core.Console;

public final class HelloWorldObjectGlobal {
    private HelloWorldObjectGlobal() {}

    static final Console console;

    static {
        console = Core.getConsole(Logger.getLogger("hello_world_object"));
        C c = new C("Hello", "World");
        console.log(c.x + ", " + c.y + "!");
        c.echo("Hello World, again!");
    }
}
