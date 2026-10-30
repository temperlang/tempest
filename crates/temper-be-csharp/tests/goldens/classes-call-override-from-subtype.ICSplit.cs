namespace CallOverrideFromSubtype;

internal interface ICSplit
{
    string A()
    {
        return ADefault(this);
    }

    protected static string ADefault(ICSplit @this)
    {
        return "C wins!";
    }
}
