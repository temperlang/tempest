using ObjectLiterals.ExtraClass;

namespace ObjectLiterals;

internal class Named : IStringable
{
    private readonly string name;

    public Named(string name)
    {
        this.name = name;
    }

    public string Name
    {
        get
        {
            return this.name;
        }
    }

    public override string ToString()
    {
        return "Named " + this.name;
    }
}
