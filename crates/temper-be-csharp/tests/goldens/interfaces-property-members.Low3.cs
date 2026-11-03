namespace PropertyMembers;

internal class Low3 : IMid
{
    public Low3()
    {
    }

    public override string ToString()
    {
        return "Low3";
    }

    public bool Eq(IHigh other)
    {
        return other is Low3;
    }
}
