using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using TemperLang.Core;

namespace Libraries;

public static class LibrariesGlobal
{
    internal static readonly Casing casingFirst;

    public static Grid ParseGrid(string s)
    {
        if (s == "")
        {
            return Core.Bubble<Grid>();
        }
        return new Grid(1, Listed.CreateReadOnlyList<double>(ParseFloat64(s)));
    }

    internal static int Rows(IReadOnlyList<double> values, int ncols)
    {
        try
        {
            return Core.Div(values.Count, ncols);
        }
        catch
        {
            Environment.FailFast("panic");
            throw new InvalidOperationException();
        }
    }

    internal static Grid Scaled(Grid grid)
    {
        if (grid.Ncols == 0)
        {
            return Core.Bubble<Grid>();
        }
        return grid;
    }

    internal static IReadOnlyList<double> Cubes(Grid grid)
    {
        try
        {
            IReadOnlyList<double> more = Listed.ToReadOnlyList(Enumerable.Select(Scaled(grid).Values, (double x) => x * x * x));
            return more;
        }
        catch
        {
            Environment.FailFast("panic");
            throw new InvalidOperationException();
        }
    }

    internal static void Each(IReadOnlyList<double> values, Action<double> f)
    {
        {
            int i = 0;
            while (i < values.Count)
            {
                f(values[i]);
                i = i + 1;
            }
        }
    }

    internal static double Total(IReadOnlyList<double> values)
    {
        double sum = 0.0;
        void fn(double x)
        {
            try
            {
                double part = x / 2.0;
                sum = sum + part;
            }
            catch
            {
                Environment.FailFast("panic");
                throw new InvalidOperationException();
            }
        }
        Each(values, fn);
        return sum;
    }

    internal static double Half(double x)
    {
        try
        {
            return x / 2.0;
        }
        catch
        {
            Environment.FailFast("panic");
            throw new InvalidOperationException();
        }
    }

    internal static double Cube(double x)
    {
        return x * 0.25;
    }

    internal static double Corner(Grid grid)
    {
        try
        {
            return grid.Get(1, 1);
        }
        catch
        {
            return -1.0;
        }
    }

    internal static Grid? Maybe(int n)
    {
        if (n > 0)
        {
            return new Grid(n, Listed.CreateReadOnlyList<double>(1.0));
        }
        else
        {
            return null;
        }
    }

    internal static IReadOnlyList<double> Mapped(Grid grid)
    {
        Grid t1;
        try
        {
            t1 = Scaled(grid);
        }
        catch
        {
            Environment.FailFast("panic");
            throw new InvalidOperationException();
        }
        double fn(double x)
        {
            if (Float64.Compare(x, 1.5) > 0)
            {
                return x * 2.0;
            }
            else
            {
                return x;
            }
        }
        return Listed.ToReadOnlyList(Enumerable.Select(t1.Values, (Func<double, double>)fn));
    }

    internal static string? Text(int n)
    {
        if (n > 0)
        {
            return "x";
        }
        else
        {
            return null;
        }
    }

    internal static int? Count(int n)
    {
        if (n > 0)
        {
            return n;
        }
        else
        {
            return null;
        }
    }

    internal static Casing? Casing(int n)
    {
        if (n > 0)
        {
            return casingFirst;
        }
        else
        {
            return null;
        }
    }

    internal static string Captured(string s)
    {
        Grid? found;
        try
        {
            found = ParseGrid(s);
        }
        catch
        {
            found = null;
        }
        return Listed.Join(Listed.ToReadOnlyList(Enumerable.Select(Listed.CreateReadOnlyList<int>(1, 2), (int x) => found == null)), ",", (bool b) => b ? "true" : "false");
    }

    internal static string Bidi()
    {
        return "look\u202Eout";
    }

    internal static string Word(int n)
    {
        if (n > 1)
        {
            return "b";
        }
        else
        {
            return "a";
        }
    }

    internal static string FormatFloat64(double d)
    {
        if (double.IsInfinity(d))
        {
            return d < 0 ? "-Infinity" : "Infinity";
        }
        if (double.IsNaN(d))
        {
            return "NaN";
        }
        string s = d.ToString(global::System.Globalization.CultureInfo.InvariantCulture);
        int e = s.IndexOf('E', global::System.StringComparison.Ordinal);
        string main = e == -1 ? s : s[..e];
        string frac = main.Contains('.', global::System.StringComparison.Ordinal) ? "" : ".0";
        string exp = e == -1 ? "" : string.Concat("e", global::System.MemoryExtensions.AsSpan(s, e + 1));
        return main + frac + exp;
    }

    internal static double ParseFloat64(string s)
    {
        string trimmed = s.Trim();
        if (trimmed.StartsWith('.') || trimmed.EndsWith('.'))
        {
            throw new global::System.FormatException();
        }
        return trimmed switch
        {
            "Infinity" => double.PositiveInfinity,
            "-Infinity" => double.NegativeInfinity,
            _ => double.Parse(
                trimmed,
                global::System.Globalization.NumberStyles.Float,
                global::System.Globalization.CultureInfo.InvariantCulture),
        };
    }

    static LibrariesGlobal()
    {
        ILoggingConsole console = Support.Logging.LoggingConsoleFactory.CreateConsole("Libraries");
        casingFirst = new Casing("first");
        Grid grid = new Grid(2, Listed.CreateReadOnlyList<double>(1.0, 2.0, 3.0, 4.0));
        console.Log(Rows(grid.Values, 2).ToString(CultureInfo.InvariantCulture) + " " + FormatFloat64(Half(3.0)) + " " + FormatFloat64(Cube(8.0)));
        console.Log(FormatFloat64(Corner(grid)));
        console.Log(FormatFloat64(Total(grid.Values)));
        string? none = Text(0);
        int? some = Count(3);
        string? part1 = none;
        int? part2 = some;
        int? part3;
        Grid? subject = Maybe(0);
        part3 = subject == null ? null : (int?)subject.Ncols;
        console.Log((part1 == null ? "null" : part1) + " " + (part2 == null ? "null" : part2.Value.ToString(CultureInfo.InvariantCulture)) + " " + (part3 == null ? "null" : part3.Value.ToString(CultureInfo.InvariantCulture)));
        console.Log((Maybe(1) != null ? "true" : "false") + " " + (Maybe(0) != null ? "true" : "false") + " " + (Grid.Parse("") == null ? "true" : "false"));
        console.Log(casingFirst.ToString());
        console.Log(Listed.Join(Mapped(grid), ",", (double x) => FormatFloat64(x)));
        console.Log(Listed.Join(Cubes(grid), ",", (double x) => FormatFloat64(x)));
        Casing? left = Casing(1);
        Casing? left2 = Casing(0);
        Casing? left3 = Casing(0);
        Casing? right = Casing(0);
        console.Log(((left == null ? false : left.Eq(casingFirst)) ? "true" : "false") + " " + ((left2 == null ? false : left2.Eq(casingFirst)) ? "true" : "false") + " " + (!(left3 == null ? right == null : right == null ? false : left3.Eq(right)) ? "true" : "false"));
        console.Log(Captured("") + " " + Captured("2.5"));
        console.Log(Bidi() != "lookout" ? "true" : "false");
        console.Log(string.Create(CultureInfo.InvariantCulture, $"{StringUtil.CompareStringsByCodePoint(Word(1), Word(2))} {StringUtil.CompareStringsByCodePoint(Word(2), Word(1))} {StringUtil.CompareStringsByCodePoint(Word(2), Word(3))}"));
        string named = Word(2);
        console.Log(named == Word(3) && named + "!" != named ? "true" : "false");
    }
}
