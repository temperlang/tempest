package broken;

import temper.core.Core;

public final class BrokenGlobal {
    private BrokenGlobal() {}

    static int undefinedPlusArgInside() {
        throw Core.bubble();
    }

    static String padLeft(String string, int minSize, String pad) {
        int needed = minSize - Core.stringCountBetween(string, 0, string.length());
        if (needed <= 0) {
            return string;
        }
        throw Core.bubble();
    }

    static {
        Core.throwBubble();
    }
}
