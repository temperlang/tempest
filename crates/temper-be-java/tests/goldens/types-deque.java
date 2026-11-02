package deque;

import java.util.ArrayDeque;
import java.util.Deque;
import java.util.logging.Logger;
import temper.core.Core;
import temper.core.Core.Console;

public final class DequeGlobal {
    private DequeGlobal() {}

    static final Console console;
    static final Deque<String> deque;

    static void checkEmpty() {
        console.log(deque.isEmpty() ? "Empty" : "Not empty");
    }

    static {
        console = Core.getConsole(Logger.getLogger("deque"));
        deque = new ArrayDeque<>();
        checkEmpty();
        if (!deque.isEmpty()) {
            console.log(Core.dequeRemoveFirst(deque));
        }
        deque.addLast("foo");
        deque.addLast("bar");
        console.log(Core.dequeRemoveFirst(deque));
        deque.addLast("baz");
        console.log(Core.dequeRemoveFirst(deque));
        deque.addLast("boo");
        checkEmpty();
        while (!deque.isEmpty()) {
            console.log(Core.dequeRemoveFirst(deque));
        }
        checkEmpty();
    }
}
