package nullables;

import temper.core.Core;

public final class NullablesMain {
    private NullablesMain() {}

    public static void main(String[] args) throws ClassNotFoundException {
        Core.initSimpleLogging();
        Class.forName("nullables.NullablesGlobal");
        Core.waitUntilTasksComplete();
    }
}
