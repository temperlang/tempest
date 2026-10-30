package private_method;

import java.util.logging.Logger;
import temper.core.Core;
import temper.core.Core.Console;

public final class PrivateMethodGlobal {
    private PrivateMethodGlobal() {}

    static {
        Console console = Core.getConsole(Logger.getLogger("private_method"));
        C c = new C();
        console.log("method1(10)=" + c.method1(10));
        console.log("method2(10)=" + c.method2(10));
    }
}
