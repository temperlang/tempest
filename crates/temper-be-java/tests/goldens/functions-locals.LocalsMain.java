package locals;

import temper.core.Core;

public final class LocalsMain {
    private LocalsMain() {}

    public static void main(String[] args) throws ClassNotFoundException {
        Core.initSimpleLogging();
        Class.forName("locals.LocalsGlobal");
        Core.waitUntilTasksComplete();
    }
}
