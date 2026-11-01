package functions;

import temper.core.Core;

public final class FunctionsMain {
    private FunctionsMain() {}

    public static void main(String[] args) throws ClassNotFoundException {
        Core.initSimpleLogging();
        Class.forName("functions.exporter.ExporterGlobal");
        Class.forName("functions.FunctionsGlobal");
        Core.waitUntilTasksComplete();
    }
}
