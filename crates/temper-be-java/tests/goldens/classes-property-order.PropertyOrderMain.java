package property_order;

import temper.core.Core;

public final class PropertyOrderMain {
    private PropertyOrderMain() {}

    public static void main(String[] args) throws ClassNotFoundException {
        Core.initSimpleLogging();
        Class.forName("property_order.PropertyOrderGlobal");
        Core.waitUntilTasksComplete();
    }
}
