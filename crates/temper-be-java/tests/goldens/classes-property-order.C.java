package property_order;

import temper.core.Nullable;

final class C {
    public final int backed;

    public C(@Nullable Integer backed) {
        int backed2;
        if (backed == null) {
            PropertyOrderGlobal.console.log("initializing backed");
            backed2 = 1;
        } else {
            backed2 = backed;
        }
        this.backed = backed2;
    }

    public C() {
        this(null);
    }

    public int getComputed() {
        PropertyOrderGlobal.console.log("computing computed");
        return 1;
    }

    public int getBacked() {
        return this.backed;
    }
}
