using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using TemperLang.Core;

namespace Functions;

public static class FunctionsGlobal
{
    internal static readonly ILoggingConsole console;

    internal static Func<int> MakeCounter(int start)
    {
        int count = start;
        int fn()
        {
            count = count + 1;
            return count;
        }
        return fn;
    }

    internal static Func<int, int> Twice(Func<int, int> f)
    {
        return (int x) => f(f(x));
    }

    internal static int ApplyTo(Func<int, int> f, int x)
    {
        return f(x);
    }

    internal static int AddOne(int x)
    {
        return x + 1;
    }

    internal static void Drive(int steps, Func<IEnumerable<Tuple<object?>>> body)
    {
        IEnumerable<Tuple<object?>> g = body();
        {
            int s = 0;
            while (s < steps)
            {
                Core.GeneratorNext(g);
                s = s + 1;
            }
        }
    }

    static FunctionsGlobal()
    {
        console = Support.Logging.LoggingConsoleFactory.CreateConsole("Functions");
        Func<int> counter = MakeCounter(10);
        counter();
        console.Log("count " + counter().ToString(CultureInfo.InvariantCulture));
        Func<int> other = MakeCounter(0);
        console.Log(string.Create(CultureInfo.InvariantCulture, $"other {other()} count {counter()}"));
        Func<int, int> addTwo = Twice(AddOne);
        console.Log(string.Create(CultureInfo.InvariantCulture, $"twice {addTwo(5)} {ApplyTo(AddOne, 1)}"));
        console.Log(string.Create(CultureInfo.InvariantCulture, $"triple {ApplyTo(Scaler.Triple, 4)} {Twice(Scaler.Triple)(2)}"));
        IReadOnlyList<int> ns = Listed.CreateReadOnlyList<int>(5, 2, 8, 1, 4);
        IReadOnlyList<int> doubled = Listed.ToReadOnlyList(Enumerable.Select(ns, (int n) => n * 2));
        IReadOnlyList<int> big = Listed.ToReadOnlyList(Enumerable.Where(doubled, (int n) => n > 4));
        console.Log(Listed.Join(big, ", ", (int n) => n.ToString(CultureInfo.InvariantCulture)));
        IReadOnlyList<int> order = Listed.Sorted(ns, (int a, int b) => a - b);
        console.Log(Listed.Join(order, " ", (int n) => n.ToString(CultureInfo.InvariantCulture)));
        int total = 0;
        {
            int length = ns.Count;
            int i2 = 0;
            while (i2 < length)
            {
                int n = ns[i2];
                i2 = i2 + 1;
                total = total + n;
            }
        }
        console.Log("total " + total.ToString(CultureInfo.InvariantCulture));
        Button button = new Button("hits", MakeCounter(0));
        button.Press();
        console.Log(button.Press());
        IList<Func<int>> makers = new List<Func<int>>();
        {
            int i = 0;
            while (i < 3)
            {
                int k = i * 10;
                makers.Add(() => k);
                i = i + 1;
            }
        }
        console.Log(Listed.Join(makers, " ", (Func<int> c) => c().ToString(CultureInfo.InvariantCulture)));
        IEnumerable<Tuple<object?>> fn8Steps1()
        {
            IList<Func<int>> fs = new List<Func<int>>();
            {
                int i = 0;
                while (i < 3)
                {
                    int j = i * 10;
                    fs.Add(() => j);
                    yield return default!;
                    fs.Add(() => j + 1);
                    i = i + 1;
                }
            }
            console.Log(Listed.Join(fs, " ", (Func<int> c) => c().ToString(CultureInfo.InvariantCulture)));
            yield return default!;
        }
        IGenerator<Tuple<object?>> fn8()
        {
            return Core.AdaptGenerator<Tuple<object?>>(fn8Steps1);
        }
        Drive(5, fn8);
    }
}
