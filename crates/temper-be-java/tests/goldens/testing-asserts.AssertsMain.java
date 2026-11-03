package asserts;

import temper.core.Core;

public final class AssertsMain {
    private AssertsMain() {}

    public static void main(String[] args) throws ClassNotFoundException {
        Core.initSimpleLogging();
        Class.forName("temper.std.testing.TestingGlobal");
        Class.forName("asserts.AssertsGlobal");
        Core.waitUntilTasksComplete();
    }
}
