namespace ContentTags;

public class Both : IEscaper
{
    private readonly IEscaper first;
    private readonly IEscaper second;

    public Both(IEscaper first, IEscaper second)
    {
        this.first = first;
        this.second = second;
    }

    public IEscaper First
    {
        get
        {
            return this.first;
        }
    }

    public IEscaper Second
    {
        get
        {
            return this.second;
        }
    }

    public string ApplyString(string x)
    {
        return this.second.ApplyString(this.first.ApplyString(x));
    }
}
