namespace CallOverrideFromSubtype;

internal interface IADeep
{
    string A()
    {
        return ADefault(this);
    }

    protected static string ADefault(IADeep @this)
    {
        return "A wins!";
    }
}
