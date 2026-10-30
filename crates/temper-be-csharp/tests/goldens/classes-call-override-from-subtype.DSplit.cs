namespace CallOverrideFromSubtype;

internal class DSplit : IBSplit, ICSplit
{
    public DSplit()
    {
    }

    public string A()
    {
        return ICSplit.ADefault(this);
    }
}
