package types;

import java.util.logging.Logger;
import temper.core.Core;
import temper.core.Core.Console;
import types.exporter.ExportedClass;

public final class TypesGlobal {
    private TypesGlobal() {}

    static {
        Console console = Core.getConsole(Logger.getLogger("types"));
        ExportedClass x = new ExportedClass(42);
        console.log("foo = " + x.foo);
    }
}
