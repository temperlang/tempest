namespace Libraries;

public class Quoting : IEscapes
{
    public Quoting()
    {
    }

    public string Escape(string? s)
    {
        return "'" + (s == null ? "null" : s) + "'";
    }

    public string EscapeFlag(bool b)
    {
        return IEscapes.EscapeFlagDefault(this, b);
    }
}
