package mutually_referencing_types;

import java.util.logging.Logger;
import temper.core.Core;
import temper.core.Core.Console;

public final class MutuallyReferencingTypesGlobal {
    private MutuallyReferencingTypesGlobal() {}

    static {
        Console console = Core.getConsole(Logger.getLogger("mutually_referencing_types"));
        Foo emptyFoo = new Foo(null);
        Bar emptyBar = new Bar(null);
        Foo nonEmptyFoo = new Foo(emptyBar);
        Bar nonEmptyBar = new Bar(emptyFoo);
        Bar stuffedBar = new Bar(nonEmptyFoo);
        console.log("emptyFoo.x has Foo -> " + (emptyFoo.x != null));
        console.log("nonEmptyFoo.x has Foo -> " + (nonEmptyFoo.x != null));
    }
}
