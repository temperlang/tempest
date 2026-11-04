package constness;

import java.util.logging.Logger;
import temper.core.Core;
import temper.core.Core.Console;

public final class ConstnessGlobal {
    private ConstnessGlobal() {}

    static {
        Console console = Core.getConsole(Logger.getLogger("constness"));
        {
            int nci = 1;
            console.log("nci = " + nci);
            nci = 2;
            console.log("nci = " + nci);
            int nc;
            nc = 1;
            console.log("nc = " + nc);
            nc = 2;
            console.log("nc = " + nc);
            {
                int i = 0;
                while (i < 3) {
                    int ncl;
                    ncl = i;
                    console.log("ncl = " + ncl);
                    i = i + 1;
                }
            }
            int ci = 1;
            console.log("ci = " + ci);
            console.log("failed to set ci");
            console.log("ci = " + ci);
            int c;
            c = 1;
            console.log("c = " + c);
            console.log("failed to set c");
            console.log("c = " + c);
            {
                int i2 = 0;
                while (i2 < 3) {
                    int cl;
                    cl = i2;
                    console.log("cl = " + cl);
                    i2 = i2 + 1;
                }
            }
        }
    }
}
