using System.Collections.Generic;
using System.Globalization;
using TemperLang.Core;

namespace Generics;

public static class GenericsGlobal
{
    internal static T Tagged<S, T>(T x)
    {
        return x;
    }

    internal static int CountOf<T>(int n)
    {
        IList<T> b = new List<T>();
        return b.Count + n;
    }

    internal static T Relayed<S, T>(T x)
        where S : IScene
    {
        return Tagged<S, T>(x);
    }

    public static T First<T>(IReadOnlyList<T> items, T fallback)
    {
        if (items.Count == 0)
        {
            return fallback;
        }
        else
        {
            return items[0];
        }
    }

    internal static T Held<T>(IHolder<T> h)
    {
        return h.Held;
    }

    internal static int Total(IReadOnlyList<int> ns)
    {
        int sum = 0;
        {
            int i = 0;
            while (i < ns.Count)
            {
                sum = sum + ns[i];
                i = i + 1;
            }
        }
        return sum;
    }

    internal static bool FirstSame(IReadOnlyList<int> a, IReadOnlyList<int> b)
    {
        return a[0] == b[0];
    }

    static GenericsGlobal()
    {
        ILoggingConsole console = Support.Logging.LoggingConsoleFactory.CreateConsole("Generics");
        IReadOnlyList<int> ns = Listed.CreateReadOnlyList<int>(1, 2, 3);
        IReadOnlyList<int> none = Listed.CreateReadOnlyList<int>();
        console.Log(string.Create(CultureInfo.InvariantCulture, $"{Total(ns)} {First(ns, 0)} {First(none, -1)}"));
        console.Log(First(Listed.CreateReadOnlyList<string>("a", "b"), "z"));
        Box<string> box = new Box<string>("boxed");
        console.Log(Held(box));
        KeyValuePair<string, int> pair = box.PairWith(7);
        console.Log(pair.Key + " " + pair.Value.ToString(CultureInfo.InvariantCulture));
        Shelf<Named> shelf = new Shelf<Named>(Listed.CreateReadOnlyList<Named>(new Named("ann"), new Named("bo")));
        string? subject = shelf.FirstName();
        console.Log(subject == null ? "none" : subject);
        IReadOnlyList<Named> noNames = Listed.CreateReadOnlyList<Named>();
        Shelf<Named> empty = new Shelf<Named>(noNames);
        string? subject2 = empty.FirstName();
        console.Log(subject2 == null ? "none" : subject2);
        int n = shelf.Tally<Named>(1) + shelf.Tally<Named>(2);
        console.Log(string.Create(CultureInfo.InvariantCulture, $"{Tagged<IScene, int>(4)} {Tagged<IScene, string>("t")} {CountOf<object>(3)} {Relayed<IScene, int>(5)} {n}"));
        IList<string> builder = new List<string>();
        builder.Add("x");
        builder.Add("y");
        builder.Insert(0, "w");
        console.Log(builder.Count.ToString(CultureInfo.InvariantCulture) + " " + builder[0] + " " + Listed.RemoveLast(builder));
        IReadOnlyList<string> built = Listed.ToReadOnlyList(builder);
        console.Log(built.Count.ToString(CultureInfo.InvariantCulture) + " " + Listed.GetOr(built, 5, "?") + " " + Listed.Slice(built, 1, 2)[0]);
        IReadOnlyDictionary<string, int> m = Mapped.ConstructMap(Listed.CreateReadOnlyList<KeyValuePair<string, int>>(new KeyValuePair<string, int>("a", 1), new KeyValuePair<string, int>("b", 2)));
        int t1;
        try
        {
            t1 = m["b"];
        }
        catch
        {
            t1 = -1;
        }
        int t2;
        try
        {
            t2 = m["c"];
        }
        catch
        {
            t2 = -1;
        }
        console.Log(string.Create(CultureInfo.InvariantCulture, $"{t1} {t2} {(m.ContainsKey("a") ? "true" : "false")} {m.Count}"));
        IDictionary<string, int?> mb = new TemperLang.Core.OrderedDictionary<string, int?>();
        mb["q"] = 5;
        mb["n"] = null;
        int? subject3 = Mapped.GetOrDefault(mb, "q", 0);
        int? subject4 = Mapped.GetOrDefault(mb, "n", 0);
        console.Log(string.Create(CultureInfo.InvariantCulture, $"{(subject3 == null ? -2 : subject3.Value)} {(subject4 == null ? -2 : subject4.Value)} {(mb.ContainsKey("n") ? "true" : "false")}"));
        string? subject5;
        int? subject6;
        try
        {
            subject6 = Core.RemoveGet(mb, "q");
        }
        catch
        {
            subject6 = 0;
        }
        subject5 = subject6 == null ? null : subject6.Value.ToString(CultureInfo.InvariantCulture);
        console.Log(subject5 == null ? "null" : subject5);
        console.Log(Mapped.Keys(mb).Count.ToString(CultureInfo.InvariantCulture));
        console.Log((FirstSame(Listed.CreateReadOnlyList<int>(1000), Listed.CreateReadOnlyList<int>(1000)) ? "true" : "false") + " " + (FirstSame(Listed.CreateReadOnlyList<int>(1000), Listed.CreateReadOnlyList<int>(7)) ? "true" : "false"));
    }
}
