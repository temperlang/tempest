package modules.helpers;

import temper.core.Core;

public final class HelpersMain {
    private HelpersMain() {}

    public static void main(String[] args) throws ClassNotFoundException {
        Core.initSimpleLogging();
        Class.forName("modules.helpers.deep.DeepGlobal");
        Class.forName("modules.helpers.HelpersGlobal");
        Core.waitUntilTasksComplete();
    }
}
