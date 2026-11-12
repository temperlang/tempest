package modules.client;

import temper.core.Core;

public final class ClientMain {
    private ClientMain() {}

    public static void main(String[] args) throws ClassNotFoundException {
        Core.initSimpleLogging();
        Core.beginTopLevel();
        try {
            Class.forName("modules.helpers.deep.DeepGlobal");
            Class.forName("modules.helpers.HelpersGlobal");
            Class.forName("modules.core.CoreGlobal");
            Class.forName("modules.ModulesGlobal");
            Class.forName("modules.client.ClientGlobal");
        } finally {
            Core.endTopLevel();
        }
        Core.waitUntilTasksComplete();
    }
}
