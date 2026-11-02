package named_args;

import temper.core.Core;

public final class NamedArgsMain {
    private NamedArgsMain() {}

    public static void main(String[] args) throws ClassNotFoundException {
        Core.initSimpleLogging();
        Class.forName("named_args.NamedArgsGlobal");
        Core.waitUntilTasksComplete();
    }
}
