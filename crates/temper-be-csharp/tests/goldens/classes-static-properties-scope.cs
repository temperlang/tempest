using TemperLang.Core;

namespace StaticPropertiesScope;

public static class StaticPropertiesScopeGlobal
{
    internal static readonly string simpleValue;
    internal static readonly string simpleOther;

    internal static string ReadSimpleValue()
    {
        return simpleValue;
    }

    internal static string ReadSimpleOther()
    {
        return simpleOther;
    }

    static StaticPropertiesScopeGlobal()
    {
        ILoggingConsole console = Support.Logging.LoggingConsoleFactory.CreateConsole("StaticPropertiesScope");
        simpleValue = "foo";
        simpleOther = "bar";
        console.Log("Simple.value is " + ReadSimpleValue());
        console.Log("Simple.other is " + ReadSimpleOther());
    }
}
