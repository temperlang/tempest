package renamed;

import temper.core.Core;

public final class RenamedMain {
    private RenamedMain() {}

    public static void main(String[] args) throws ClassNotFoundException {
        Core.initSimpleLogging();
        Class.forName("renamed.RenamedGlobal");
        Core.waitUntilTasksComplete();
    }
}
