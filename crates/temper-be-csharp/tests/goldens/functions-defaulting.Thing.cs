namespace Defaulting;

internal class Thing
{
    public Thing()
    {
    }

    public void Say(string? a = null, string? b = null)
    {
        string a2 = a ?? "";
        string b2 = b ?? "";
        DefaultingGlobal.console.Log("saying " + a2 + b2);
    }

    public static void SayStatic(string a, string? b = null)
    {
        string b2 = b ?? "";
        DefaultingGlobal.console.Log("static " + a + b2);
    }
}
