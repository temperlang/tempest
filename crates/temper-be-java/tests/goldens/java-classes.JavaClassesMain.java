package java_classes;

import temper.core.Core;

public final class JavaClassesMain {
    private JavaClassesMain() {}

    public static void main(String[] args) throws ClassNotFoundException {
        Core.initSimpleLogging();
        Class.forName("java_classes.JavaClassesGlobal");
        Core.waitUntilTasksComplete();
    }
}
