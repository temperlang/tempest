package pure_virtual;

import java.util.logging.Logger;
import temper.core.Core;
import temper.core.Core.Console;

public final class PureVirtualGlobal {
    private PureVirtualGlobal() {}

    static final Console console;

    static {
        console = Core.getConsole(Logger.getLogger("pure_virtual"));
        Greeter greeter = new HiGreeter();
        greeter.greet();
    }
}
