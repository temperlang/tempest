using Functions.Exporter;
using System;
using System.Collections.Generic;
using System.Linq;
using TemperLang.Core;

namespace Functions;

public static class FunctionsGlobal
{
    public static double Norm(double x, double y)
    {
        return Math.Sqrt(ExporterGlobal.NormSquared(x, y));
    }

    public static double Something(IReadOnlyList<double> vals)
    {
        return Enumerable.Aggregate(vals, 0.0, (Func<double, double, double>)ExporterGlobal.NormSquared);
    }

    internal static string FormatFloat64(double d)
    {
        if (double.IsInfinity(d))
        {
            return d < 0 ? "-Infinity" : "Infinity";
        }
        if (double.IsNaN(d))
        {
            return "NaN";
        }
        string s = d.ToString(global::System.Globalization.CultureInfo.InvariantCulture);
        int e = s.IndexOf('E', global::System.StringComparison.Ordinal);
        string main = e == -1 ? s : s[..e];
        string frac = main.Contains('.', global::System.StringComparison.Ordinal) ? "" : ".0";
        string exp = e == -1 ? "" : string.Concat("e", s.AsSpan(e + 1));
        return main + frac + exp;
    }

    static FunctionsGlobal()
    {
        ILoggingConsole console = Support.Logging.LoggingConsoleFactory.CreateConsole("Functions");
        double a = 3.0;
        double b = a + 1.0;
        string values = Listed.Join(Listed.CreateReadOnlyList<double>(a, b, Norm(a, b)), ", ", (double it) => FormatFloat64(it));
        console.Log(values + " triangle");
        console.Log(FormatFloat64(Something(Listed.CreateReadOnlyList<double>(a, b))));
    }
}
