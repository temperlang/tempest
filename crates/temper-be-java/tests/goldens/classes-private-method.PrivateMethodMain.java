package private_method;

import temper.core.Core;

public final class PrivateMethodMain {
    private PrivateMethodMain() {}

    public static void main(String[] args) throws ClassNotFoundException {
        Core.initSimpleLogging();
        Class.forName("private_method.PrivateMethodGlobal");
        Core.waitUntilTasksComplete();
    }
}
