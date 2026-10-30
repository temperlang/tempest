package classes;

import temper.core.Core;

public final class ClassesMain {
    private ClassesMain() {}

    public static void main(String[] args) throws ClassNotFoundException {
        Core.initSimpleLogging();
        Class.forName("classes.ClassesGlobal");
        Core.waitUntilTasksComplete();
    }
}
