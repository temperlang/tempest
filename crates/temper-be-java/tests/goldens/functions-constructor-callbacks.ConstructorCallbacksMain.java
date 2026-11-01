package constructor_callbacks;

import temper.core.Core;

public final class ConstructorCallbacksMain {
    private ConstructorCallbacksMain() {}

    public static void main(String[] args) throws ClassNotFoundException {
        Core.initSimpleLogging();
        Class.forName("constructor_callbacks.ConstructorCallbacksGlobal");
        Core.waitUntilTasksComplete();
    }
}
