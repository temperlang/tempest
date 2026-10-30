package setters;

import temper.core.Core;

public final class SettersMain {
    private SettersMain() {}

    public static void main(String[] args) throws ClassNotFoundException {
        Core.initSimpleLogging();
        Class.forName("setters.SettersGlobal");
        Core.waitUntilTasksComplete();
    }
}
