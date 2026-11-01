using System;
using TemperLang.Core;

namespace AsValues;

public static class AsValuesGlobal
{
    internal static readonly ILoggingConsole console;

    internal static void Hello(string how)
    {
        console.Log("Called " + how + "!");
    }

    internal static void CallIt(Action<string> f)
    {
        f("by a function");
    }

    internal static bool IsOdd(int i)
    {
        if (i == 0)
        {
            return false;
        }
        else
        {
            return IsEven(i - 1);
        }
    }

    internal static bool IsEven(int i)
    {
        if (i == 0)
        {
            return true;
        }
        else
        {
            return IsOdd(i - 1);
        }
    }

    static AsValuesGlobal()
    {
        console = Support.Logging.LoggingConsoleFactory.CreateConsole("AsValues");
        Hello("directly");
        CallIt(Hello);
        new DelayedFunction(Hello).CallIt();
        console.Log("Is 5 even? " + (IsEven(5) ? "true" : "false"));
        console.Log("Is 6 even? " + (IsEven(6) ? "true" : "false"));
    }
}
