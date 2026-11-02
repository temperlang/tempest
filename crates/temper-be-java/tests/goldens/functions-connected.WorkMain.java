package work;

import temper.core.Core;

public final class WorkMain {
    private WorkMain() {}

    public static void main(String[] args) throws ClassNotFoundException {
        Core.initSimpleLogging();
        Class.forName("work.WorkGlobal");
        Core.waitUntilTasksComplete();
    }
}
