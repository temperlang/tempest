package generics;

import java.util.List;
import temper.core.Core;
import temper.core.Nullable;

public final class Shelf<T extends Named> {
    public final List<T> items;

    public Shelf(List<T> items) {
        this.items = items;
    }

    public @Nullable String firstName() {
        if (this.items.isEmpty()) {
            return null;
        } else {
            return Core.listGet(this.items, 0).name;
        }
    }

    public List<T> getItems() {
        return this.items;
    }
}
