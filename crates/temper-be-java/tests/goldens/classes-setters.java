package setters;

import java.util.logging.Logger;
import temper.core.Core;
import temper.core.Core.Console;

public final class SettersGlobal {
    private SettersGlobal() {}

    static {
        Console console = Core.getConsole(Logger.getLogger("setters"));
        console.log(new Foo("initial").getBar());
        Foo foo = new Foo("initial");
        foo.setBar("next");
        console.log(foo.getBar());
        Bar bar = new Bar("initial");
        bar.usingSet("next");
        console.log(bar.getBar());
        Whatever what = new ConcreteWhatever(4);
        what.setSomething(5);
        console.log("something set to " + what.getSomething());
    }
}
