using TemperLang.Core;

namespace IfReturn;

public static class IfReturnGlobal
{
    internal static string Explicit(bool b)
    {
        if (b)
        {
            return "true";
        }
        return "false";
    }

    internal static string HalfAndHalf(bool b)
    {
        if (b)
        {
            return "true";
        }
        return "false";
    }

    internal static string Implicit(bool b)
    {
        if (b)
        {
            return "true";
        }
        else
        {
            return "false";
        }
    }

    static IfReturnGlobal()
    {
        ILoggingConsole console = Support.Logging.LoggingConsoleFactory.CreateConsole("IfReturn");
        bool t = true;
        bool f = !t;
        console.Log("explicit    true  -> " + Explicit(t));
        console.Log("explicit    false -> " + Explicit(f));
        console.Log("halfAndHalf true  -> " + HalfAndHalf(t));
        console.Log("halfAndHalf false -> " + HalfAndHalf(f));
        console.Log("implicit    true  -> " + Implicit(t));
        console.Log("implicit    false -> " + Implicit(f));
    }
}
