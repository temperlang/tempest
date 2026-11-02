package work;

import java.util.logging.Logger;
import temper.core.Core;
import temper.core.Core.Console;

public final class WorkGlobal {
    private WorkGlobal() {}

    static {
        Console console = Core.getConsole(Logger.getLogger("work"));
        Counter counter = new Counter();
        counter.add(2);
        counter.add(3);
        console.log("counter.total(): " + counter.total());
        console.log("new Doubler().twice(5): " + new Doubler().twice(5));
    }
}
