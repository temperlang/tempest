namespace CallOverrideFromSubtype;

internal interface IC : IA
{
    new string A()
    {
        return ADefault(this);
    }

    protected static string ADefault(IC @this)
    {
        return "C wins!";
    }

    new string Thing
    {
        get
        {
            return GetThingDefault(this);
        }
    }

    protected static string GetThingDefault(IC @this)
    {
        return "C has thing!";
    }
}
