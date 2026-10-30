namespace CallOverrideFromSubtype;

internal interface IA
{
    string A()
    {
        return ADefault(this);
    }

    protected static string ADefault(IA @this)
    {
        return "A wins!";
    }

    string Thing
    {
        get
        {
            return GetThingDefault(this);
        }
    }

    protected static string GetThingDefault(IA @this)
    {
        return "A has thing!";
    }
}
