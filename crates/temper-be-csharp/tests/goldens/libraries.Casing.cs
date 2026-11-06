namespace Libraries;

public class Casing
{
    private readonly string name;

    public bool Eq(Casing other)
    {
        return this.name == other.name;
    }

    public Casing(string name)
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
        return this.name;
    }

    public static Casing First
    {
        get
        {
            return LibrariesGlobal.casingFirst;
        }
    }
}
