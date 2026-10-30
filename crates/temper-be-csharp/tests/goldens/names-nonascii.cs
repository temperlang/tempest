using TemperLang.Core;

namespace Nonascii;

public static class NonasciiGlobal
{
    internal static readonly ILoggingConsole console;

    internal static void SayHello(string όνομα)
    {
        console.Log("Hello, " + όνομα + "!");
    }

    static NonasciiGlobal()
    {
        console = Support.Logging.LoggingConsoleFactory.CreateConsole("Nonascii");
        SayHello("World");
    }
}
