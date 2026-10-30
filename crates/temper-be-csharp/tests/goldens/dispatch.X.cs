namespace Dispatch;

internal class X : IC
{
    public X()
    {
    }

    public string A()
    {
        return IC.ADefault(this);
    }

    public string B
    {
        get
        {
            return IC.GetBDefault(this);
        }
    }
}
