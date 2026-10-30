package static_properties;

import temper.core.Core;

public final class StaticPropertiesMain {
    private StaticPropertiesMain() {}

    public static void main(String[] args) throws ClassNotFoundException {
        Core.initSimpleLogging();
        Class.forName("static_properties.StaticPropertiesGlobal");
        Core.waitUntilTasksComplete();
    }
}
