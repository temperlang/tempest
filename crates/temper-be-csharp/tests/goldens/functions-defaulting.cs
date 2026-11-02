using System;
using System.Globalization;
using TemperLang.Core;

namespace Defaulting;

public static class DefaultingGlobal
{
    internal static readonly ILoggingConsole console;
    internal static volatile int fCalls;
    internal static volatile int inked;
    private static readonly object moduleLock = new();

    internal static int F()
    {
        lock (moduleLock)
        {
            fCalls = fCalls + 1;
        }
        return 42;
    }

    internal static int G(int? x = null)
    {
        int x2 = x ?? F();
        return x2;
    }

    internal static int? NoInline()
    {
        console.Log("noInline");
        return null;
    }

    internal static string MaybeEmpty(StringHolder? a)
    {
        lock (moduleLock)
        {
            inked = inked + 1;
        }
        {
            string? subject;
            bool? subject2;
            string? subject3 = a == null ? null : a.String;
            subject2 = subject3 == null ? null : (bool?)(subject3.Length == 0);
            subject = subject2 == null ? null : subject2.Value ? "true" : "false";
            if (subject == null)
            {
                return "null";
            }
            else
            {
                return subject;
            }
        }
    }

    internal static int? MaybeLength(string? a)
    {
        lock (moduleLock)
        {
            inked = inked + 1;
        }
        {
            if (a == null)
            {
                return null;
            }
            else
            {
                return StringUtil.CountBetween(a, 0, a.Length);
            }
        }
    }

    internal static int? MaybeInc()
    {
        if (inked == 10)
        {
            return null;
        }
        else
        {
            lock (moduleLock)
            {
                inked = inked + 1;
            }
            return inked;
        }
    }

    internal static string ChainMore(string? a)
    {
        int? subject1;
        int? subject2 = MaybeLength(a);
        subject1 = subject2 == null ? null : (int?)Math.Max(subject2.Value, 0);
        return (subject1 == null ? -1 : subject1.Value).ToString(CultureInfo.InvariantCulture);
    }

    static DefaultingGlobal()
    {
        console = Support.Logging.LoggingConsoleFactory.CreateConsole("Defaulting");
        fCalls = 0;
        console.Log("fCalls=" + fCalls.ToString(CultureInfo.InvariantCulture));
        console.Log("g(   )=" + G().ToString(CultureInfo.InvariantCulture));
        console.Log("fCalls=" + fCalls.ToString(CultureInfo.InvariantCulture));
        console.Log("g(123)=" + G(123).ToString(CultureInfo.InvariantCulture));
        console.Log("fCalls=" + fCalls.ToString(CultureInfo.InvariantCulture));
        console.Log("g(null)=" + G(NoInline()).ToString(CultureInfo.InvariantCulture));
        console.Log("fCalls=" + fCalls.ToString(CultureInfo.InvariantCulture));
        new Thing().Say("hi");
        new Thing().Say(null, "there");
        new Thing().Say("by", "e");
        Thing.SayStatic("yall");
        inked = 0;
        int? subject4 = MaybeInc();
        console.Log("maybeInc() -> " + (subject4 == null ? -1 : subject4.Value).ToString(CultureInfo.InvariantCulture));
        console.Log("inked: " + inked.ToString(CultureInfo.InvariantCulture));
        console.Log("lengthish \"ab\": " + ChainMore("ab"));
        console.Log("lengthish null: " + ChainMore(null));
        console.Log("inked: " + inked.ToString(CultureInfo.InvariantCulture));
        console.Log("maybeEmpty \"\": " + MaybeEmpty(new StringHolder("")));
        console.Log("maybeEmpty null: " + MaybeEmpty(null));
        console.Log("inked: " + inked.ToString(CultureInfo.InvariantCulture));
    }
}
