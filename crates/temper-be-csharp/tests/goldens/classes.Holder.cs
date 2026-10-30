namespace Classes;

internal class Holder
{
    private readonly Loose loose;
    private readonly string tag;

    public Holder(Loose loose, string tag)
    {
        this.loose = loose;
        this.tag = tag;
    }

    public Loose Loose
    {
        get
        {
            return this.loose;
        }
    }

    public string Tag
    {
        get
        {
            return this.tag;
        }
    }

    public bool Eq(Holder other)
    {
        if (this.loose.Eq(other.loose))
        {
            return this.tag == other.tag;
        }
        else
        {
            return false;
        }
    }
}
