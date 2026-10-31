package generics;

import temper.core.Core;

public final class GenericsMain {
    private GenericsMain() {}

    public static void main(String[] args) throws ClassNotFoundException {
        Core.initSimpleLogging();
        Class.forName("generics.GenericsGlobal");
        Core.waitUntilTasksComplete();
    }
}
