using System.Globalization;
using TemperLang.Core;

namespace StaticProperties;

public static class StaticPropertiesGlobal
{
    internal static readonly ILoggingConsole console;
    internal static readonly Simple simpleSimon;
    internal static readonly string simpleValue;
    internal static readonly int simpleOther;

    static StaticPropertiesGlobal()
    {
        console = Support.Logging.LoggingConsoleFactory.CreateConsole("StaticProperties");
        simpleSimon = new Simple("foo");
        simpleValue = simpleSimon.Name;
        simpleOther = 22;
        console.Log("Simple.simon.name is " + simpleSimon.Name);
        console.Log("Simple.value is " + simpleValue);
        console.Log("Simple.other is " + simpleOther.ToString(CultureInfo.InvariantCulture));
        C.SayHi();
        II.SayBye();
    }
}
