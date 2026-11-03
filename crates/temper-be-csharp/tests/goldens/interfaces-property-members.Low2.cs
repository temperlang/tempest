namespace PropertyMembers;

internal class Low2 : IMid
{
    private readonly string s;

    public Low2(string s)
    {
        this.s = s;
    }

    public string S
    {
        get
        {
            return this.s;
        }
    }

    public override string ToString()
    {
        return "Low2(" + this.s + ")";
    }

    public bool Eq(IHigh other)
    {
        if (other is Low2)
        {
            return this.s == ((Low2)other).s;
        }
        else
        {
            return false;
        }
    }
}
