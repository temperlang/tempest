package pure_virtual;

import temper.core.Core;

public final class PureVirtualMain {
    private PureVirtualMain() {}

    public static void main(String[] args) throws ClassNotFoundException {
        Core.initSimpleLogging();
        Class.forName("pure_virtual.PureVirtualGlobal");
        Core.waitUntilTasksComplete();
    }
}
