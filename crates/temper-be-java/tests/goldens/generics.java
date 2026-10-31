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

    static {
        Console console = Core.getConsole(Logger.getLogger("generics"));
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
    }
}
