namespace StaticProperties;

internal class C
{
    public C()
    {
    }

    public static void SayHi()
    {
        StaticPropertiesGlobal.console.Log(Compose("C", "'hi.'"));
    }

    private static string Compose(string subject, string message)
    {
        return subject + " says " + message;
    }
}
