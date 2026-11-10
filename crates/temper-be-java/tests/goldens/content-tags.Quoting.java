package content_tags;

import java.util.List;
import java.util.function.BiConsumer;
import java.util.function.BiPredicate;
import temper.core.Core;

public final class Quoting implements ContextualAutoescapingAccumulator {
    private boolean inQuote;
    private final Buffer collector;

    public static Buffer newCollector() {
        return new Buffer();
    }

    public static boolean initialState() {
        return false;
    }

    public static Propagator propagator() {
        return new Propagator("'");
    }

    public static Picker picker() {
        return new Picker(false);
    }

    public static BiPredicate<Boolean, Boolean> sameStatePredicate() {
        return (a, b) -> a == b;
    }

    public static boolean mergeStates(boolean a, boolean b, BiConsumer<String, Boolean> callout) {
        if (a != b) {
            callout.accept("branches end in different contexts", true);
        }
        return a;
    }

    public static void checkEndState(boolean inQuote, BiConsumer<String, Boolean> callout) {
        if (inQuote) {
            callout.accept("ends inside quotes", true);
        }
    }

    public static String fromCollector(Buffer c) {
        return c.toString();
    }

    public static void enact(Mark m, Buffer c) {
        c.mark(m);
    }

    public Quoting() {
        this.inQuote = false;
        this.collector = new Buffer();
    }

    public void appendSafe(String x) {
        After after = ContentTagsGlobal.propagateOver(propagator(), this.inQuote, x, sameStatePredicate(), ContentTagsGlobal::ignore);
        this.inQuote = after.stateAfter;
        {
            List<Effect> list = after.effects;
            int length = list.size();
            int i = 0;
            while (i < length) {
                Effect e = Core.listGet(list, i);
                i = i + 1;
                e.enactOn(this.collector);
            }
        }
    }

    public void append(String x) {
        String escaped = picker().escaperFor(this.inQuote, ContentTagsGlobal::ignore).applyString(x);
        this.collector.append(escaped);
    }

    public String getAccumulated() {
        return this.collector.toString();
    }
}
