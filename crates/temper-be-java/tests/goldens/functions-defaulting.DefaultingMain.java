package defaulting;

import temper.core.Core;

public final class DefaultingMain {
    private DefaultingMain() {}

    public static void main(String[] args) throws ClassNotFoundException {
        Core.initSimpleLogging();
        Class.forName("defaulting.DefaultingGlobal");
        Core.waitUntilTasksComplete();
    }
}
