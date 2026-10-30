namespace CallOverrideFromSubtype;

internal class EDeep : ICDeep, IB2_Deep
{
    public EDeep()
    {
    }

    public string A()
    {
        return IADeep.ADefault(this);
    }
}
