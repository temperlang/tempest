package modules.helpers.deep;

import temper.core.Core;

public final class DeepMain {
    private DeepMain() {}

    public static void main(String[] args) throws ClassNotFoundException {
        Core.initSimpleLogging();
        Class.forName("modules.helpers.deep.DeepGlobal");
        Core.waitUntilTasksComplete();
    }
}
