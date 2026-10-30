package jumps;

import java.util.logging.Logger;
import temper.core.Core;
import temper.core.Core.Console;

public final class JumpsGlobal {
    private JumpsGlobal() {}

    static int search(int n) {
        int found = -1;
        int i = 0;
        outer: while (i < n) {
            int j = 0;
            while (true) {
                if (j >= n) {
                    break;
                }
                if (i * j == 6) {
                    found = i * 10 + j;
                    break outer;
                }
                j = j + 1;
            }
            i = i + 1;
        }
        return found;
    }

    static int odds(int n) {
        int k = 0;
        int sum = 0;
        do {
            k = k + 1;
            if ((k & 1) == 0) {
                continue;
            }
            sum = sum + k;
        } while (k < n);
        return sum;
    }

    static {
        Console console = Core.getConsole(Logger.getLogger("jumps"));
        console.log(Integer.toString(search(5)));
        console.log(Integer.toString(odds(9)));
    }
}
