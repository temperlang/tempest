package sorting;

import temper.core.Core;

public final class SortingMain {
    private SortingMain() {}

    public static void main(String[] args) throws ClassNotFoundException {
        Core.initSimpleLogging();
        Class.forName("sorting.SortingGlobal");
        Core.waitUntilTasksComplete();
    }
}
