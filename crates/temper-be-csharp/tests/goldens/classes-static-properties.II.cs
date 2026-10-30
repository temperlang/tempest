namespace StaticProperties;

internal interface II
{
    public static void SayBye()
    {
        StaticPropertiesGlobal.console.Log("I says 'bye.'");
    }
}
