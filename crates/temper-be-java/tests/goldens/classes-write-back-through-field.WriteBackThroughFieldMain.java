package write_back_through_field;

import temper.core.Core;

public final class WriteBackThroughFieldMain {
    private WriteBackThroughFieldMain() {}

    public static void main(String[] args) throws ClassNotFoundException {
        Core.initSimpleLogging();
        Class.forName("write_back_through_field.WriteBackThroughFieldGlobal");
        Core.waitUntilTasksComplete();
    }
}
