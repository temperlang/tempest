package modules.core;

import temper.core.Core;

public final class CoreGlobal {
    private CoreGlobal() {}

    public static int share(int n, int d) {
        try {
            return Core.divIntInt(n, d);
        } catch (RuntimeException ignored) {
            return 0;
        }
    }
}
