package inherited_getter;

import temper.core.Core;

public final class InheritedGetterMain {
    private InheritedGetterMain() {}

    public static void main(String[] args) throws ClassNotFoundException {
        Core.initSimpleLogging();
        Class.forName("inherited_getter.InheritedGetterGlobal");
        Core.waitUntilTasksComplete();
    }
}
