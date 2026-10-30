package is_empty;

import java.util.logging.Logger;
import temper.core.Core;
import temper.core.Core.Console;

public final class IsEmptyGlobal {
    private IsEmptyGlobal() {}

    static {
        Console console = Core.getConsole(Logger.getLogger("is_empty"));
        String emptyString = "";
        String foo = "foo";
        console.log(emptyString.isEmpty() ? "emptyString is empty" : "emptyString is not empty");
        console.log(foo.isEmpty() ? "foo is empty" : "foo is not empty");
    }
}
