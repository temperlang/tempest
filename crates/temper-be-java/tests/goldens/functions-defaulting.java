package defaulting;

import java.util.logging.Logger;
import temper.core.Core;
import temper.core.Core.Console;
import temper.core.Nullable;

public final class DefaultingGlobal {
    private DefaultingGlobal() {}

    static final Console console;
    static volatile int fCalls;
    static volatile int inked;

    static int f() {
        synchronized (DefaultingGlobal.class) {
            fCalls = fCalls + 1;
        }
        return 42;
    }

    static int g(@Nullable Integer x) {
        int x2 = x != null ? x : f();
        return x2;
    }

    static int g() {
        return g(null);
    }

    static @Nullable Integer noInline() {
        console.log("noInline");
        return null;
    }

    static String maybeEmpty(@Nullable StringHolder a) {
        synchronized (DefaultingGlobal.class) {
            inked = inked + 1;
        }
        {
            @Nullable String subject3 = a == null ? null : a.string;
            @Nullable Boolean subject2 = subject3 == null ? null : Boolean.valueOf(subject3.isEmpty());
            @Nullable String subject = subject2 == null ? null : Boolean.toString(subject2);
            if (subject == null) {
                return "null";
            } else {
                return subject;
            }
        }
    }

    static @Nullable Integer maybeLength(@Nullable String a) {
        synchronized (DefaultingGlobal.class) {
            inked = inked + 1;
        }
        if (a == null) {
            return null;
        } else {
            return Core.stringCountBetween(a, 0, a.length());
        }
    }

    static @Nullable Integer maybeInc() {
        if (inked == 10) {
            return null;
        } else {
            synchronized (DefaultingGlobal.class) {
                inked = inked + 1;
            }
            return inked;
        }
    }

    static String chainMore(@Nullable String a) {
        @Nullable Integer subject2 = maybeLength(a);
        @Nullable Integer subject = subject2 == null ? null : Integer.valueOf(Math.max(subject2, 0));
        return Integer.toString(subject == null ? -1 : subject);
    }

    static {
        console = Core.getConsole(Logger.getLogger("defaulting"));
        fCalls = 0;
        console.log("fCalls=" + fCalls);
        console.log("g(   )=" + g());
        console.log("fCalls=" + fCalls);
        console.log("g(123)=" + g(123));
        console.log("fCalls=" + fCalls);
        console.log("g(null)=" + g(noInline()));
        console.log("fCalls=" + fCalls);
        new Thing().say("hi");
        new Thing().say(null, "there");
        new Thing().say("by", "e");
        Thing.sayStatic("yall");
        inked = 0;
        @Nullable Integer subject = maybeInc();
        console.log("maybeInc() -> " + (subject == null ? -1 : subject));
        console.log("inked: " + inked);
        console.log("lengthish \"ab\": " + chainMore("ab"));
        console.log("lengthish null: " + chainMore(null));
        console.log("inked: " + inked);
        console.log("maybeEmpty \"\": " + maybeEmpty(new StringHolder("")));
        console.log("maybeEmpty null: " + maybeEmpty(null));
        console.log("inked: " + inked);
    }
}
