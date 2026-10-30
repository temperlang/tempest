namespace CallOverrideFromSubtype;

internal class D : IB, IC
{
    public D()
    {
    }

    public string A()
    {
        return IC.ADefault(this);
    }

    public string Thing
    {
        get
        {
            return IC.GetThingDefault(this);
        }
    }
}
