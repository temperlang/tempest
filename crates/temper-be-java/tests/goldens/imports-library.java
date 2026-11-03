package imports_library;

import java.util.logging.Logger;
import modules.ModulesGlobal;
import modules.helpers.HelpersGlobal;
import renamed.Checked;
import renamed.RenamedGlobal;
import temper.core.Core;
import temper.core.Core.Console;

public final class ImportsLibraryGlobal {
    private ImportsLibraryGlobal() {}

    public static int both(int n) {
        return ModulesGlobal.sum(n) + HelpersGlobal.twice(n) + HelpersGlobal.base;
    }

    static {
        Console console = Core.getConsole(Logger.getLogger("imports_library"));
        console.log("both " + both(2));
        Checked checked = new Checked(RenamedGlobal.lambda(RenamedGlobal.native_));
        console.log("renamed " + checked.delete() + " " + checked.getLocal() + " " + checked.end);
    }
}
