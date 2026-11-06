package modules;

import temper.core.Core;

public final class ModulesMain {
    private ModulesMain() {}

    public static void main(String[] args) throws ClassNotFoundException {
        Core.initSimpleLogging();
        Core.beginTopLevel();
        try {
            Class.forName("modules.helpers.deep.DeepGlobal");
            Class.forName("modules.helpers.HelpersGlobal");
            Class.forName("modules.core.CoreGlobal");
            Class.forName("modules.ModulesGlobal");
        } finally {
            Core.endTopLevel();
        }
        Core.waitUntilTasksComplete();
    }
}
