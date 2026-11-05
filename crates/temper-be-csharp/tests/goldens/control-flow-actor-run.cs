using System;
using System.Collections.Generic;
using System.Globalization;
using TemperLang.Core;

namespace ActorRun;

public static class ActorRunGlobal
{
    internal static readonly ILoggingConsole console;

    internal static void F(Func<IEnumerable<Tuple<object?>>> body)
    {
        IEnumerable<Tuple<object?>> generator = body();
        console.Log("f0");
        Core.GeneratorNext(generator);
        console.Log("f1");
        Core.GeneratorNext(generator);
        console.Log("f2");
        Core.GeneratorNext(generator);
        console.Log("f3");
    }

    static ActorRunGlobal()
    {
        console = Support.Logging.LoggingConsoleFactory.CreateConsole("ActorRun");
        IEnumerable<Tuple<object?>> fnSteps1()
        {
            int i = 0;
            while (true)
            {
                console.Log("generated " + i.ToString(CultureInfo.InvariantCulture));
                i = i + 1;
                yield return default!;
            }
        }
        IGenerator<Tuple<object?>> fn()
        {
            return Core.AdaptGenerator<Tuple<object?>>(fnSteps1);
        }
        F(fn);
    }
}
