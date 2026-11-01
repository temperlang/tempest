package functions;

import java.util.ArrayList;
import java.util.List;
import java.util.function.IntSupplier;
import java.util.function.IntUnaryOperator;
import java.util.logging.Logger;
import temper.core.Core;
import temper.core.Core.Console;

public final class FunctionsGlobal {
    private FunctionsGlobal() {}

    static IntSupplier makeCounter(int start) {
        class CountCell {
            int value;
        }
        final CountCell count = new CountCell();
        count.value = start;
        IntSupplier fn = () -> {
            count.value = count.value + 1;
            return count.value;
        };
        return fn;
    }

    static IntUnaryOperator twice(IntUnaryOperator f) {
        return x -> f.applyAsInt(f.applyAsInt(x));
    }

    static int applyTo(IntUnaryOperator f, int x) {
        return f.applyAsInt(x);
    }

    static int addOne(int x) {
        return x + 1;
    }

    static {
        Console console = Core.getConsole(Logger.getLogger("functions"));
        IntSupplier counter = makeCounter(10);
        counter.getAsInt();
        console.log("count " + counter.getAsInt());
        IntSupplier other = makeCounter(0);
        console.log("other " + other.getAsInt() + " count " + counter.getAsInt());
        IntUnaryOperator addTwo = twice(FunctionsGlobal::addOne);
        console.log("twice " + addTwo.applyAsInt(5) + " " + applyTo(FunctionsGlobal::addOne, 1));
        console.log("triple " + applyTo(Scaler::triple, 4) + " " + twice(Scaler::triple).applyAsInt(2));
        List<Integer> ns = List.of(5, 2, 8, 1, 4);
        List<Integer> doubled = Core.listMapIntToInt(ns, n -> n * 2);
        List<Integer> big = Core.listFilterInt(doubled, n -> n > 4);
        console.log(Core.listJoinInt(big, ", ", n -> Integer.toString(n)));
        List<Integer> order = Core.listSortedInt(ns, (a, b) -> a - b);
        console.log(Core.listJoinInt(order, " ", n -> Integer.toString(n)));
        int total = 0;
        {
            int length = ns.size();
            int i2 = 0;
            while (i2 < length) {
                int n = Core.listGet(ns, i2);
                i2 = i2 + 1;
                total = total + n;
            }
        }
        console.log("total " + total);
        Button button = new Button("hits", makeCounter(0));
        button.press();
        console.log(button.press());
        List<IntSupplier> makers = new ArrayList<>();
        {
            int i = 0;
            while (i < 3) {
                int k = i * 10;
                makers.add(() -> k);
                i = i + 1;
            }
        }
        console.log(Core.listJoinObj(makers, " ", c -> Integer.toString(c.getAsInt())));
    }
}
