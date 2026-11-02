package build;

import temper.core.Core;

public final class BuildMain {
    private BuildMain() {}

    public static void main(String[] args) throws ClassNotFoundException {
        Core.initSimpleLogging();
        Class.forName("build.BuildGlobal");
        Core.waitUntilTasksComplete();
    }
}
