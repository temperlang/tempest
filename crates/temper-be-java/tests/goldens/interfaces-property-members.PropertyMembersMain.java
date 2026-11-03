package property_members;

import temper.core.Core;

public final class PropertyMembersMain {
    private PropertyMembersMain() {}

    public static void main(String[] args) throws ClassNotFoundException {
        Core.initSimpleLogging();
        Class.forName("property_members.PropertyMembersGlobal");
        Core.waitUntilTasksComplete();
    }
}
