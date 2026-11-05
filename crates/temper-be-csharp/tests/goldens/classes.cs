using System;
using System.Globalization;
using TemperLang.Core;

namespace Classes;

public static class ClassesGlobal
{
    internal static readonly Rect rectUnit;

    internal static IShape Shape(IShape x)
    {
        return x;
    }

    internal static Type TypeNamed(int n)
    {
        {
            if (n == 0)
            {
                return typeof(void);
            }
            else if (n == 1)
            {
                return typeof(bool);
            }
            else if (n == 2)
            {
                return typeof(int);
            }
            else if (n == 3)
            {
                return typeof(long);
            }
            else if (n == 4)
            {
                return typeof(double);
            }
            else
            {
                return typeof(string);
            }
        }
    }

    internal static Type Same(Type t)
    {
        return t;
    }

    static ClassesGlobal()
    {
        ILoggingConsole console = Support.Logging.LoggingConsoleFactory.CreateConsole("Classes");
        rectUnit = new Rect(1, 1);
        Rect r = new Rect(2, 3);
        console.Log(r.ToString());
        console.Log(r.Area.ToString(CultureInfo.InvariantCulture));
        IShape s = Shape(r);
        console.Log(s.Describe());
        IShape q = Shape(new Square(4));
        console.Log(q.Describe());
        console.Log(q.Area.ToString(CultureInfo.InvariantCulture));
        console.Log(r.Scaled(2).Scaled(3).Area.ToString(CultureInfo.InvariantCulture));
        console.Log(rectUnit.Scaled(5).ToString());
        Counter c = Counter.Zero();
        c.Bump().Bump();
        console.Log(c.Count.ToString(CultureInfo.InvariantCulture));
        c.Doubled = 10;
        console.Log(c.Doubled.ToString(CultureInfo.InvariantCulture));
        console.Log(c.Same(new Counter().Bump().Bump()) ? "true" : "false");
        console.Log((new Point(1, 2).Eq(new Point(1, 2)) ? "true" : "false") + " " + (!new Point(1, 2).Eq(new Point(2, 1)) ? "true" : "false"));
        console.Log(new Holder(new Loose(3), "a").Eq(new Holder(new Loose(-3), "a")) ? "true" : "false");
        console.Log((new Version(1, 2).CompareTo(new Version(1, 3)) < 0 ? "true" : "false") + " " + (new Version(2, 0).CompareTo(new Version(1, 9)) >= 0 ? "true" : "false"));
        console.Log((q is Square ? "true" : "false") + " " + (s is Square ? "true" : "false"));
        Square sq = (Square)q;
        console.Log(sq.Area.ToString(CultureInfo.InvariantCulture));
        if (s is Rect)
        {
            console.Log(((Rect)s).W.ToString(CultureInfo.InvariantCulture));
        }
        Type favorite = Same(typeof(long));
        int named = 0;
        {
            int i = 0;
            while (i < 6)
            {
                TypeNamed(i);
                named = named + 1;
                i = i + 1;
            }
        }
        console.Log(named.ToString(CultureInfo.InvariantCulture) + " types");
    }
}
