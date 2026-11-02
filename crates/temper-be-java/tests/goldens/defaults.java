package defaults;

import defaults.helpers.HelpersGlobal;
import defaults.helpers.Pad;
import java.util.logging.Logger;
import temper.core.Core;
import temper.core.Core.Console;
import temper.core.Nullable;

public final class DefaultsGlobal {
    private DefaultsGlobal() {}

    static volatile int calls;

    static int next() {
        synchronized (DefaultsGlobal.class) {
            calls = calls + 1;
        }
        return calls;
    }

    static String range(int start, @Nullable Integer end, @Nullable Integer step) {
        int end2 = end != null ? end : start + 10;
        int step2 = step != null ? step : next();
        return start + ".." + end2 + " by " + step2;
    }

    static String range(int start, @Nullable Integer end) {
        return range(start, end, null);
    }

    static String range(int start) {
        return range(start, null, null);
    }

    static String tag(String text, @Nullable String mark) {
        if (mark == null) {
            return text;
        } else {
            return mark + text;
        }
    }

    static String tag(String text) {
        return tag(text, null);
    }

    static String named(@Nullable String name) {
        @Nullable String name2 = name != null ? name : "nobody";
        if (name2 == null) {
            return "null";
        } else {
            return name2;
        }
    }

    static String named() {
        return named(null);
    }

    static {
        Console console = Core.getConsole(Logger.getLogger("defaults"));
        calls = 0;
        console.log(range(1));
        console.log(range(1, 5));
        console.log(range(1, 5, 3));
        console.log(range(2, null, 7));
        console.log("calls " + calls);
        console.log(tag("plain"));
        console.log(tag("marked", "#"));
        console.log(named());
        console.log(named(null));
        console.log(named("Ada"));
        console.log(new Greeter().greet());
        console.log(new Greeter("Hi").greet("you", 2));
        console.log(new Greeter(null, "?").greet(null, 2));
        console.log(Greeter.shout("hey"));
        Counter counter = new Counter();
        counter.add();
        console.log("count " + counter.add(5) + " " + new Counter(1).add());
        console.log(HelpersGlobal.wrap("a"));
        console.log(HelpersGlobal.wrap("b", "<"));
        console.log(HelpersGlobal.wrap("c", "<", ">"));
        Pad pad = new Pad();
        console.log(pad.apply("x"));
        console.log(new Pad(0, 3).apply("y", "-"));
    }
}
