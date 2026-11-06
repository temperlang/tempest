using System.Collections.Generic;
using System.Globalization;
using TemperLang.Core;

namespace Classes;

internal class Sql
{
    private readonly IList<string> parts;

    public Sql()
    {
        this.parts = new List<string>();
    }

    public void AppendSafe(string s)
    {
        this.parts.Add(s);
    }

    public void AppendString(string? x)
    {
        this.parts.Add(x == null ? "NULL" : "'" + x + "'");
    }

    public void AppendInt(int x)
    {
        this.parts.Add(x.ToString(CultureInfo.InvariantCulture));
    }

    public void AppendShape(IShape x)
    {
        this.parts.Add("shape " + x.Area.ToString(CultureInfo.InvariantCulture));
    }

    public void AppendRect(Rect x)
    {
        this.parts.Add("rect " + x.ToString());
    }

    public string Accumulated
    {
        get
        {
            return Listed.Join(this.parts, "", (string p) => p);
        }
    }
}
