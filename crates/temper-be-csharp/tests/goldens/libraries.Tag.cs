namespace Libraries;

public class Tag : IShown
{
    private readonly string name;

    public Tag(string name)
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
        return "<" + this.name + ">";
    }
}
