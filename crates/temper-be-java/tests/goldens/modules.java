package modules;

import java.util.logging.Logger;
import modules.core.CoreGlobal;
import modules.helpers.HelpersGlobal;
import modules.helpers.deep.DeepGlobal;
import modules.machine.MachineGlobal;
import temper.core.Core;
import temper.core.Core.Console;

public final class ModulesGlobal {
    private ModulesGlobal() {}

    static final int base2;

    public static int sum(int n) {
        return HelpersGlobal.twice(n) + HelpersGlobal.base + base2;
    }

    static {
        Console console = Core.getConsole(Logger.getLogger("modules"));
        int calls2 = HelpersGlobal.calls;
        base2 = 1;
        console.log(DeepGlobal.greet("modules"));
        console.log("sum " + sum(3));
        console.log("share " + CoreGlobal.share(7, 2) + " " + HelpersGlobal.intDoubled(21));
        console.log(new Engine("own").name + ", " + MachineGlobal.describe());
        HelpersGlobal.bump();
        HelpersGlobal.bump();
        console.log("calls " + calls2);
        console.log("calls now " + HelpersGlobal.bump());
    }
}
