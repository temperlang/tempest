package types;

import temper.core.Core;

public final class TypesMain {
    private TypesMain() {}

    public static void main(String[] args) throws ClassNotFoundException {
        Core.initSimpleLogging();
        Class.forName("types.exporter.ExporterGlobal");
        Class.forName("types.TypesGlobal");
        Core.waitUntilTasksComplete();
    }
}
