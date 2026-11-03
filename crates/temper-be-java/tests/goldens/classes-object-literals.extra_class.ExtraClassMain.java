package object_literals.extra_class;

import temper.core.Core;

public final class ExtraClassMain {
    private ExtraClassMain() {}

    public static void main(String[] args) throws ClassNotFoundException {
        Core.initSimpleLogging();
        Class.forName("object_literals.extra_class.ExtraClassGlobal");
        Core.waitUntilTasksComplete();
    }
}
