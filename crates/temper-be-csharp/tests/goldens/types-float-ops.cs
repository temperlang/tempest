using System;
using TemperLang.Core;

namespace Ops;

public static class OpsGlobal
{
    internal static readonly ILoggingConsole console;
    internal static readonly double two;

    internal static void Near(string label, double actual, double expected)
    {
        string result;
        bool subject = Float64.Near(actual, expected);
        result = subject == true ? "✅" : "❌ wanted " + FormatFloat64(expected) + ", was " + FormatFloat64(actual);
        console.Log(label + ": " + result);
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
        string exp = e == -1 ? "" : string.Concat("e", global::System.MemoryExtensions.AsSpan(s, e + 1));
        return main + frac + exp;
    }

    static OpsGlobal()
    {
        console = Support.Logging.LoggingConsoleFactory.CreateConsole("Ops");
        double one = 1.0;
        two = one + one;
        double half = one / two;
        Near("half.abs()", Math.Abs(half), half);
        Near("(-half).abs()", Math.Abs(-half), half);
        double three = one + two;
        Near("half.acos()", Math.Acos(half), Math.PI / three);
        double pi = Math.PI;
        double six = two * three;
        Near("half.asin()", Math.Asin(half), pi / six);
        double four = two * two;
        Near("(-one).atan()", Math.Atan(-one), -pi / four);
        Near("one.atan2(-one)", Math.Atan2(one, -one), (three * pi) / four);
        Near("(four / three).ceil()", Math.Ceiling(four / three), two);
        Near("(-four / three).ceil()", Math.Ceiling(-four / three), -one);
        Near("(pi / three).cos()", Math.Cos(pi / three), half);
        double e = Math.E;
        Near("one.cosh()", Math.Cosh(one), (e + one / e) / two);
        Near("half.exp() == e.sqrt()", Math.Exp(half), Math.Sqrt(e));
        Near("half.expm1()", Float64.ExpM1(half), Math.Sqrt(e) - one);
        Near("(four / three).floor()", Math.Floor(four / three), one);
        Near("(-four / three).floor()", Math.Floor(-four / three), -two);
        Near("e.log()", Math.Log(Math.E), one);
        double five = one + four;
        double ten = two * five;
        Near("ten.log10()", Math.Log10(ten), one);
        Near("(e - one).log1p()", Float64.LogP1(e - one), one);
        string fn(double x)
        {
            string fn2(double y)
            {
                string pair = Listed.Join(Listed.CreateReadOnlyList<double>(Math.Max(x, y), Math.Min(x, y)), ", ", (double z) => FormatFloat64(z));
                return "(" + pair + ")";
            }
            return Listed.Join(Listed.CreateReadOnlyList<double>(double.NaN, two), ", ", (Func<double, string>)fn2);
        }
        string maxMinCombos = Listed.Join(Listed.CreateReadOnlyList<double>(double.NaN, one), ", ", (Func<double, string>)fn);
        console.Log("maxMinCombos: " + maxMinCombos);
        double quarter = half / two;
        Near("2.25 % 2.0", (two + quarter) % two, 0.25);
        Near("2.25 % -2.0", (two + quarter) % -two, 0.25);
        Near("-2.25 % 2.0", -(two + quarter) % two, -0.25);
        Near("-2.25 % -2.0", -(two + quarter) % -two, -0.25);
        Near("pi.round()", Math.Round(pi), three);
        Near("(-pi).round()", Math.Round(-pi), -three);
        Near("pi.sign()", Float64.Sign(pi), one);
        Near("(-pi).sign()", Float64.Sign(-pi), -one);
        double zero = one - one;
        console.Log("zero.sign(): " + FormatFloat64(Float64.Sign(zero)));
        console.Log("(-zero).sign(): " + FormatFloat64(Float64.Sign(-zero)));
        Near("(pi / six).sin()", Math.Sin(pi / six), half);
        Near("one.sinh()", Math.Sinh(one), (e - one / e) / two);
        Near("(-pi / four).tan()", Math.Tan(-pi / four), -one);
        Near("(three * pi / four).tan()", Math.Tan((three * pi) / four), -one);
        Near("one.tanh()", Math.Tanh(one), (Math.Pow(e, 2.0) - 1.0) / (Math.Pow(e, 2.0) + 1.0));
        console.Log("one.near(one + 0.1): " + (Float64.Near(one, one + 0.1) ? "true" : "false"));
        console.Log("one.near(one + 0.25, absTol = 0.25): " + (Float64.Near(one, one + 0.25, null, 0.25) ? "true" : "false"));
        console.Log("one.near(one + 0.1 + 1e-9, absTol = 0.1): " + (Float64.Near(one, one + 0.1 + 1e-9, null, 0.1) ? "true" : "false"));
        console.Log("one.near(one - 0.1, absTol = 0.11): " + (Float64.Near(one, one - 0.1, null, 0.11) ? "true" : "false"));
        console.Log("ten.near(ten + 0.1, absTol = 0.011): " + (Float64.Near(ten, ten + 0.1, null, 0.011) ? "true" : "false"));
        console.Log("ten.near(ten + 0.1, relTol = 0.011): " + (Float64.Near(ten, ten + 0.1, 0.011) ? "true" : "false"));
    }
}
