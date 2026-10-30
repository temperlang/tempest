using System.Globalization;
using TemperLang.Core;

namespace Fibonacci;

public static class FibonacciGlobal
{
    internal static int Fib(int i)
    {
        int a = 0;
        int b = 1;
        while (i > 0)
        {
            int c = a + b;
            a = b;
            b = c;
            i = i - 1;
        }
        return a;
    }

    static FibonacciGlobal()
    {
        ILoggingConsole console = Support.Logging.LoggingConsoleFactory.CreateConsole("Fibonacci");
        int zero = 0;
        console.Log("fib(1)=" + Fib(zero + 1).ToString(CultureInfo.InvariantCulture));
        console.Log("fib(10)=" + Fib(zero + 10).ToString(CultureInfo.InvariantCulture));
        console.Log("fib(20)=" + Fib(zero + 20).ToString(CultureInfo.InvariantCulture));
    }
}
