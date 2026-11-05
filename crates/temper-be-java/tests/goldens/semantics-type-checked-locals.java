package type_checked_locals;

import java.util.logging.Logger;
import temper.core.Core;
import temper.core.Core.Console;

public final class TypeCheckedLocalsGlobal {
    private TypeCheckedLocalsGlobal() {}

    static final Console console;

    static void yay() {
        console.log("yay");
    }

    static void meh() {
        console.log("meh");
    }

    static void boo() {
        console.log("boo");
    }

    static int f(int x) {
        return x;
    }

    static int g(int x) {
        return x;
    }

    static int i(int x) {
        return x;
    }

    static {
        console = Core.getConsole(Logger.getLogger("type_checked_locals"));
        console.log("Checkpoint 0");
        try {
            int a;
            Core.throwBubble();
        } catch (RuntimeException ignored) {
            yay();
        }
        console.log("Checkpoint 1");
        int a2 = 1;
        yay();
        console.log("Checkpoint 2");
        {
            int a3 = 1;
            try {
                Core.throwBubble();
            } catch (RuntimeException ignored2) {
                meh();
            }
            if (a3 == 1) {
                meh();
            } else {
                meh();
            }
        }
        console.log("Checkpoint 3");
        int a4 = 1;
        a4 = 2;
        yay();
        console.log("Checkpoint 4");
        try {
            f(0);
            yay();
        } catch (RuntimeException ignored3) {
            boo();
        }
        try {
            Core.throwBubble();
        } catch (RuntimeException ignored4) {
            meh();
        }
        console.log("Checkpoint 5");
        try {
            g(0);
            yay();
        } catch (RuntimeException ignored5) {
            boo();
        }
        try {
            Core.throwBubble();
        } catch (RuntimeException ignored6) {
            meh();
        }
        console.log("Checkpoint 6");
        try {
            i(0);
            yay();
        } catch (RuntimeException ignored7) {
            boo();
        }
        try {
            Core.throwBubble();
        } catch (RuntimeException ignored8) {
            meh();
        }
    }
}
