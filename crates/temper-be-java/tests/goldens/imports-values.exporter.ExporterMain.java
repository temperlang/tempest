package values.exporter;

import temper.core.Core;

public final class ExporterMain {
    private ExporterMain() {}

    public static void main(String[] args) throws ClassNotFoundException {
        Core.initSimpleLogging();
        Class.forName("values.exporter.ExporterGlobal");
        Core.waitUntilTasksComplete();
    }
}
