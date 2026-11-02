package defaults;

import temper.core.Core;

public final class DefaultsMain {
    private DefaultsMain() {}

    public static void main(String[] args) throws ClassNotFoundException {
        Core.initSimpleLogging();
        Class.forName("defaults.helpers.HelpersGlobal");
        Class.forName("defaults.DefaultsGlobal");
        Core.waitUntilTasksComplete();
    }
}
