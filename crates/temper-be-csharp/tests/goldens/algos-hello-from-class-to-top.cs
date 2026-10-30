using TemperLang.Core;

namespace HelloFromClassToTop;

public static class HelloFromClassToTopGlobal
{
    internal static readonly ILoggingConsole console;

    internal static void Say(string message)
    {
        console.Log(message);
    }

    static HelloFromClassToTopGlobal()
    {
        console = Support.Logging.LoggingConsoleFactory.CreateConsole("HelloFromClassToTop");
        Something something = new Something("Hello, World!");
        something.SayMessage();
    }
}
