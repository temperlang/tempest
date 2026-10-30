package direct_getter;

import temper.core.Core;

public final class DirectGetterMain {
    private DirectGetterMain() {}

    public static void main(String[] args) throws ClassNotFoundException {
        Core.initSimpleLogging();
        Class.forName("direct_getter.DirectGetterGlobal");
        Core.waitUntilTasksComplete();
    }
}
