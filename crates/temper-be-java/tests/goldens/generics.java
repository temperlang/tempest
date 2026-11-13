package generics;

import java.util.AbstractMap.SimpleImmutableEntry;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.Map.Entry;
import java.util.logging.Logger;
import temper.core.Core;
import temper.core.Core.Console;
import temper.core.Nullable;

public final class GenericsGlobal {
    private GenericsGlobal() {}

    static final Console console;
    static final Map<String, Integer> tally;

    static <S, T> T tagged(T x) {
        return x;
    }

    static <T> int countOf(int n) {
        List<T> b = new ArrayList<>();
        return b.size() + n;
    }

    static <S extends Scene, T> T relayed(T x) {
        return tagged(x);
    }

    public static <T> T first(List<T> items, T fallback) {
        if (items.isEmpty()) {
            return fallback;
        } else {
            return Core.listGet(items, 0);
        }
    }

    static <T> T held(Holder<T> h) {
        return h.getHeld();
    }

    static int total(List<Integer> ns) {
        int sum = 0;
        {
            int i = 0;
            while (i < ns.size()) {
                sum = sum + Core.listGet(ns, i);
                i = i + 1;
            }
        }
        return sum;
    }

    static void indexed() {
        List<Integer> counts = new ArrayList<>();
        counts.add(1);
        counts.add(10);
        int at = 0;
        {
            int k = at;
            counts.set(k, Core.listGet(counts, k) + 4);
        }
        counts.set(1, Core.listGet(counts, 1) * 3);
        int t = at;
        at = t + 1;
        int k2 = t;
        int t2 = Core.listGet(counts, k2);
        counts.set(k2, t2 + 1);
        int was = t2;
        int k3 = at;
        int t3 = Core.listGet(counts, k3) + 1;
        counts.set(k3, t3);
        int pre = t3;
        int t4 = Core.listGet(counts, 0);
        counts.set(0, t4 - 1);
        int post = t4;
        int t5 = Core.listGet(counts, 0) - 1;
        counts.set(0, t5);
        int down = t5;
        console.log(was + " " + pre + " " + post + " " + down + " " + at);
    }

    static int bump(String key) {
        tally.put(key, Core.mappedGet(tally, key) - 1);
        int t = Core.mappedGet(tally, key);
        tally.put(key, t + 1);
        int before = t;
        int t2 = Core.mappedGet(tally, key) + 1;
        tally.put(key, t2);
        int after = t2;
        return before + after;
    }

    static boolean firstSame(List<Integer> a, List<Integer> b) {
        return Core.listGet(a, 0).intValue() == Core.listGet(b, 0);
    }

    static {
        console = Core.getConsole(Logger.getLogger("generics"));
        List<Integer> ns = List.of(1, 2, 3);
        List<Integer> none = List.of();
        console.log(total(ns) + " " + first(ns, 0) + " " + first(none, -1));
        console.log(first(List.of("a", "b"), "z"));
        Box<String> box = new Box<String>("boxed");
        console.log(held(box));
        Entry<String, Integer> pair = box.pairWith(7);
        console.log(pair.getKey() + " " + pair.getValue());
        Shelf<Named> shelf = new Shelf<Named>(List.of(new Named("ann"), new Named("bo")));
        @Nullable String subject = shelf.firstName();
        console.log(subject == null ? "none" : subject);
        List<Named> noNames = List.of();
        Shelf<Named> empty = new Shelf<Named>(noNames);
        @Nullable String subject2 = empty.firstName();
        console.log(subject2 == null ? "none" : subject2);
        int n = shelf.tally(1) + shelf.tally(2);
        console.log(tagged(4) + " " + tagged("t") + " " + countOf(3) + " " + relayed(5) + " " + n);
        List<String> builder = new ArrayList<>();
        builder.add("x");
        builder.add("y");
        builder.add(0, "w");
        console.log(builder.size() + " " + Core.listGet(builder, 0) + " " + Core.listRemoveLast(builder));
        List<String> built = List.copyOf(builder);
        console.log(built.size() + " " + Core.listGetOr(built, 5, "?") + " " + Core.listGet(Core.listSlice(built, 1, 2), 0));
        Map<String, Integer> m = Core.mapConstructor(List.of(new SimpleImmutableEntry<>("a", 1), new SimpleImmutableEntry<>("b", 2)));
        int t;
        try {
            t = Core.mappedGet(m, "b");
        } catch (RuntimeException ignored) {
            t = -1;
        }
        String t2 = t + " ";
        int t3;
        try {
            t3 = Core.mappedGet(m, "c");
        } catch (RuntimeException ignored2) {
            t3 = -1;
        }
        console.log(t2 + t3 + " " + m.containsKey("a") + " " + m.size());
        Map<String, @Nullable Integer> mb = new LinkedHashMap<>();
        mb.put("q", 5);
        mb.put("n", null);
        @Nullable Integer subject3 = mb.getOrDefault("q", 0);
        String t4 = (subject3 == null ? -2 : subject3) + " ";
        @Nullable Integer subject4 = mb.getOrDefault("n", 0);
        console.log(t4 + (subject4 == null ? -2 : subject4) + " " + mb.containsKey("n"));
        @Nullable Integer subject6;
        try {
            subject6 = Core.mapBuilderRemove(mb, "q");
        } catch (RuntimeException ignored3) {
            subject6 = 0;
        }
        @Nullable String subject5 = subject6 == null ? null : Integer.toString(subject6);
        console.log(subject5 == null ? "null" : subject5);
        console.log(Integer.toString(List.copyOf(mb.keySet()).size()));
        indexed();
        tally = new LinkedHashMap<>();
        tally.put("k", 2);
        int t5;
        try {
            t5 = bump("k");
        } catch (RuntimeException ignored4) {
            t5 = -1;
        }
        String t6 = t5 + " ";
        int t7;
        try {
            t7 = bump("z");
        } catch (RuntimeException ignored5) {
            t7 = -1;
        }
        String t8 = t6 + t7 + " ";
        int t9;
        try {
            t9 = Core.mappedGet(tally, "k");
        } catch (RuntimeException ignored6) {
            t9 = -1;
        }
        console.log(t8 + t9);
        console.log(firstSame(List.of(1000), List.of(1000)) + " " + firstSame(List.of(1000), List.of(7)));
    }
}
