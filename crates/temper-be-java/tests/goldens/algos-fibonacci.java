package fibonacci;

import java.util.logging.Logger;
import temper.core.Core;
import temper.core.Core.Console;

public final class FibonacciGlobal {
    private FibonacciGlobal() {}

    static int fib(int i) {
        int a = 0;
        int b = 1;
        while (i > 0) {
            int c = a + b;
            a = b;
            b = c;
            i = i - 1;
        }
        return a;
    }

    static {
        Console console = Core.getConsole(Logger.getLogger("fibonacci"));
        int zero = 0;
        zero = zero;
        console.log("fib(1)=" + fib(zero + 1));
        console.log("fib(10)=" + fib(zero + 10));
        console.log("fib(20)=" + fib(zero + 20));
    }
}
