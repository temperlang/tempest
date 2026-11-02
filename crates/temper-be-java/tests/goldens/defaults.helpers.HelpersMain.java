package defaults.helpers;

import temper.core.Core;

public final class HelpersMain {
    private HelpersMain() {}

    public static void main(String[] args) throws ClassNotFoundException {
        Core.initSimpleLogging();
        Class.forName("defaults.helpers.HelpersGlobal");
        Core.waitUntilTasksComplete();
    }
}
