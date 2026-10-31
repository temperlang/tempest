using System.Globalization;
using TemperLang.Core;
using Values.Exporter;

namespace Values;

public static class ValuesGlobal
{
    public static int TimesMagic(int n)
    {
        return n * ExporterGlobal.MagicNumber;
    }

    static ValuesGlobal()
    {
        ILoggingConsole console = Support.Logging.LoggingConsoleFactory.CreateConsole("Values");
        int a = 1;
        int c = TimesMagic(a);
        console.Log("number = " + c.ToString(CultureInfo.InvariantCulture));
    }
}
