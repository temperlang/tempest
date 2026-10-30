using TemperLang.Core;

namespace PureVirtual;

public static class PureVirtualGlobal
{
    internal static readonly ILoggingConsole console;

    static PureVirtualGlobal()
    {
        console = Support.Logging.LoggingConsoleFactory.CreateConsole("PureVirtual");
        IGreeter greeter = new HiGreeter();
        greeter.Greet();
    }
}
