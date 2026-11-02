namespace Defaults.Helpers;

public static class HelpersGlobal
{
    public static string Wrap(string text, string? open = null, string? close = null)
    {
        string open2 = open ?? "[";
        string close2 = close ?? open2;
        return open2 + text + close2;
    }
}
