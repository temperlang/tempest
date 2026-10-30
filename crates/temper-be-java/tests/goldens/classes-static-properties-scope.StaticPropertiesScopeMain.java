package static_properties_scope;

import temper.core.Core;

public final class StaticPropertiesScopeMain {
    private StaticPropertiesScopeMain() {}

    public static void main(String[] args) throws ClassNotFoundException {
        Core.initSimpleLogging();
        Class.forName("static_properties_scope.StaticPropertiesScopeGlobal");
        Core.waitUntilTasksComplete();
    }
}
