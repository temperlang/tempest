using System;
using System.Collections.Generic;
using TemperLang.Core;

namespace ContentTags;

public static class ContentTagsGlobal
{
    internal static readonly ILoggingConsole console;
    internal static readonly TextEscaper textEscaperInstance;
    internal static readonly QuoteEscaper quoteEscaperInstance;

    internal static void Ignore(string problem, bool isError)
    {
        if (isError)
        {
            console.Log(problem);
        }
    }

    public static After PropagateOver(Propagator p, bool before, string? known, Func<bool, bool, bool> same, Action<string, bool> callout)
    {
        if (known == null)
        {
            return new After(Listed.CreateReadOnlyList<IEffect>(), before);
        }
        if (known == "!")
        {
            return new After(Listed.CreateReadOnlyList<IEffect>(new EventParseEffect(new Mark("bang"))), before);
        }
        int angles = Core.Split(known, "<").Count;
        if (angles > 1)
        {
            callout("angle bracket in fixed text", false);
        }
        if (!same(before, before))
        {
            callout("states differ from themselves", true);
        }
        int quotes = Core.Split(known, p.Quote).Count - 1;
        return new After(Listed.CreateReadOnlyList<IEffect>(new AppendParseEffect(known)), Core.ModSafe(quotes, 2) == 1 ? !before : before);
    }

    internal static string Greet(string name, string who)
    {
        {
            Buffer collector = Quoting.NewCollector();
            {
                collector.Append("say 'hi ");
                collector.Append(quoteEscaperInstance.ApplyString(textEscaperInstance.ApplyString(name)));
                collector.Append("' to ");
                collector.Append(textEscaperInstance.ApplyString(who));
                Quoting.Enact(new Mark("bang"), collector);
            }
            return Quoting.FromCollector(collector);
        }
    }

    internal static string Branch(bool c, string x)
    {
        {
            Buffer collector = Quoting.NewCollector();
            {
                collector.Append(textEscaperInstance.ApplyString(x));
                if (c)
                {
                    collector.Append("''");
                }
                collector.Append(" '");
                collector.Append(quoteEscaperInstance.ApplyString(textEscaperInstance.ApplyString(x)));
                collector.Append("'");
            }
            return Quoting.FromCollector(collector);
        }
    }

    internal static string Each(IReadOnlyList<string> xs)
    {
        {
            Quoting accumulator = new Quoting();
            {
                {
                    int length = xs.Count;
                    int i = 0;
                    while (i < length)
                    {
                        string x = xs[i];
                        i = i + 1;
                        accumulator.AppendSafe("'");
                        accumulator.Append(x);
                        accumulator.AppendSafe("' ");
                        accumulator.Append(x);
                        accumulator.AppendSafe(";");
                    }
                }
            }
            return accumulator.Accumulated;
        }
    }

    static ContentTagsGlobal()
    {
        console = Support.Logging.LoggingConsoleFactory.CreateConsole("ContentTags");
        textEscaperInstance = new TextEscaper();
        quoteEscaperInstance = new QuoteEscaper();
        console.Log(Greet("Ann", "Bob"));
        console.Log(Branch(true, "a"));
        console.Log(Branch(false, "b"));
        console.Log(Each(Listed.CreateReadOnlyList<string>("c", "d")));
    }
}
