using System.Globalization;
using TemperLang.Core;
using Types.Exporter;

namespace Types;

public static class TypesGlobal
{
    static TypesGlobal()
    {
        ILoggingConsole console = Support.Logging.LoggingConsoleFactory.CreateConsole("Types");
        ExportedClass x = new ExportedClass(42);
        console.Log("foo = " + x.Foo.ToString(CultureInfo.InvariantCulture));
    }
}
