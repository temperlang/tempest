package testing;

import temper.core.Core;

public final class TestingMain {
    private TestingMain() {}

    public static void main(String[] args) throws ClassNotFoundException {
        Core.initSimpleLogging();
        Class.forName("testing.TestingGlobal");
        Core.waitUntilTasksComplete();
    }
}
