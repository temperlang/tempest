using System.Globalization;

namespace PropertyMembers;

internal class Low1 : IHigh
{
    private readonly int i;

    public Low1(int i)
    {
        this.i = i;
    }

    public int I
    {
        get
        {
            return this.i;
        }
    }

    public override string ToString()
    {
        return "Low1(" + this.i.ToString(CultureInfo.InvariantCulture) + ")";
    }

    public bool Eq(IHigh other)
    {
        if (other is Low1)
        {
            return this.i == ((Low1)other).i;
        }
        else
        {
            return false;
        }
    }
}
