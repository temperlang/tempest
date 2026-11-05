package locals;

import java.util.ArrayList;
import java.util.List;
import java.util.function.Consumer;
import temper.core.Core;

final class Hub<T> {
    private final List<Consumer<T>> handlers;

    public Hub() {
        this.handlers = new ArrayList<>();
    }

    public void onAction(Consumer<T> handler) {
        this.handlers.add(handler);
    }

    public void actionPerformed(T item) {
        {
            int i = 0;
            while (i < this.handlers.size()) {
                Core.listGet(this.handlers, i).accept(item);
                i = i + 1;
            }
        }
    }
}
