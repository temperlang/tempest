package specific;

import temper.core.Core;

public final class SpecificMain {
    private SpecificMain() {}

    public static void main(String[] args) throws ClassNotFoundException {
        Core.initSimpleLogging();
        Class.forName("specific.SpecificGlobal");
        Core.waitUntilTasksComplete();
    }
}
