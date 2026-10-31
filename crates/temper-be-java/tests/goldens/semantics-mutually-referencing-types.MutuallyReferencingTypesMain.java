package mutually_referencing_types;

import temper.core.Core;

public final class MutuallyReferencingTypesMain {
    private MutuallyReferencingTypesMain() {}

    public static void main(String[] args) throws ClassNotFoundException {
        Core.initSimpleLogging();
        Class.forName("mutually_referencing_types.MutuallyReferencingTypesGlobal");
        Core.waitUntilTasksComplete();
    }
}
