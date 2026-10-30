package call_override_from_subtype;

import temper.core.Core;

public final class CallOverrideFromSubtypeMain {
    private CallOverrideFromSubtypeMain() {}

    public static void main(String[] args) throws ClassNotFoundException {
        Core.initSimpleLogging();
        Class.forName("call_override_from_subtype.CallOverrideFromSubtypeGlobal");
        Core.waitUntilTasksComplete();
    }
}
