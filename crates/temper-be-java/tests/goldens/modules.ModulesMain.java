package modules;

import temper.core.Core;

public final class ModulesMain {
    private ModulesMain() {}

    public static void main(String[] args) throws ClassNotFoundException {
        Core.initSimpleLogging();
        Class.forName("modules.helpers.deep.DeepGlobal");
        Class.forName("modules.helpers.HelpersGlobal");
        Class.forName("modules.ModulesGlobal");
        Core.waitUntilTasksComplete();
    }
}
