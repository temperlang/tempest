package empty;

import temper.core.Core;

public final class EmptyMain {
    private EmptyMain() {}

    public static void main(String[] args) throws ClassNotFoundException {
        Core.initSimpleLogging();
        Class.forName("empty.EmptyGlobal");
        Core.waitUntilTasksComplete();
    }
}
