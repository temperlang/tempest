package hello_world_object;

import temper.core.Core;

public final class HelloWorldObjectMain {
    private HelloWorldObjectMain() {}

    public static void main(String[] args) throws ClassNotFoundException {
        Core.initSimpleLogging();
        Class.forName("hello_world_object.HelloWorldObjectGlobal");
        Core.waitUntilTasksComplete();
    }
}
