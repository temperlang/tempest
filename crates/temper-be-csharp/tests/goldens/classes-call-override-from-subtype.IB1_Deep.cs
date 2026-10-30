namespace CallOverrideFromSubtype;

internal interface IB1_Deep : IADeep
{
    new string A()
    {
        return ADefault(this);
    }

    protected static string ADefault(IB1_Deep @this)
    {
        return "B1 wins!";
    }
}
