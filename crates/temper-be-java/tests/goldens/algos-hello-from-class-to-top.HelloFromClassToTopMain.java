package hello_from_class_to_top;

import temper.core.Core;

public final class HelloFromClassToTopMain {
    private HelloFromClassToTopMain() {}

    public static void main(String[] args) throws ClassNotFoundException {
        Core.initSimpleLogging();
        Class.forName("hello_from_class_to_top.HelloFromClassToTopGlobal");
        Core.waitUntilTasksComplete();
    }
}
