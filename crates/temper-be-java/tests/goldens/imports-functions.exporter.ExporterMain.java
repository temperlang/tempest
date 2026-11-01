package functions.exporter;

import temper.core.Core;

public final class ExporterMain {
    private ExporterMain() {}

    public static void main(String[] args) throws ClassNotFoundException {
        Core.initSimpleLogging();
        Class.forName("functions.exporter.ExporterGlobal");
        Core.waitUntilTasksComplete();
    }
}
