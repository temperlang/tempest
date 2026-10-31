package values;

import java.util.logging.Logger;
import temper.core.Core;
import temper.core.Core.Console;
import values.exporter.ExporterGlobal;

public final class ValuesGlobal {
    private ValuesGlobal() {}

    public static int timesMagic(int n) {
        return n * ExporterGlobal.magicNumber;
    }

    static {
        Console console = Core.getConsole(Logger.getLogger("values"));
        int a = 1;
        a = a;
        int c = timesMagic(a);
        console.log("number = " + c);
    }
}
