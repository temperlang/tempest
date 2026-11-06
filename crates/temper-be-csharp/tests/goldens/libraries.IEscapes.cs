namespace Libraries;

public interface IEscapes
{
    string Escape(string? s);

    string EscapeFlag(bool b)
    {
        return EscapeFlagDefault(this, b);
    }

    protected static string EscapeFlagDefault(IEscapes @this, bool b)
    {
        return @this.Escape(b ? "T" : "F");
    }
}
