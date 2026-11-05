package angle_call;

import java.util.List;
import temper.core.Core;

final class Classy {
    public final int index;

    public Classy(int index) {
        this.index = index;
    }

    public static <T> T first(List<T> things) {
        T thing = Core.listGet(things, 0);
        return thing;
    }

    public <T> T indexed(List<T> things) {
        T thing = Core.listGet(things, this.index);
        return thing;
    }

    public int getIndex() {
        return this.index;
    }
}
