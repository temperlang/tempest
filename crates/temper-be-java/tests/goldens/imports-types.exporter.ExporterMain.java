package types.exporter;

import temper.core.Core;

public final class ExporterMain {
    private ExporterMain() {}

    public static void main(String[] args) throws ClassNotFoundException {
        Core.initSimpleLogging();
        Class.forName("types.exporter.ExporterGlobal");
        Core.waitUntilTasksComplete();
    }
}
