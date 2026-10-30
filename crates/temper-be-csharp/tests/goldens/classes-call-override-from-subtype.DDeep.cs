namespace CallOverrideFromSubtype;

internal class DDeep : IB2_Deep, ICDeep
{
    public DDeep()
    {
    }

    public string A()
    {
        return IB1_Deep.ADefault(this);
    }
}
