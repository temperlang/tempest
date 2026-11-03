package object_literals;

import temper.core.Core;

public final class ObjectLiteralsMain {
    private ObjectLiteralsMain() {}

    public static void main(String[] args) throws ClassNotFoundException {
        Core.initSimpleLogging();
        Class.forName("object_literals.extra_class.ExtraClassGlobal");
        Class.forName("object_literals.ObjectLiteralsGlobal");
        Core.waitUntilTasksComplete();
    }
}
