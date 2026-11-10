namespace ContentTags;

public class Propagator
{
    private readonly string quote;

    public Propagator(string quote)
    {
        this.quote = quote;
    }

    public string Quote
    {
        get
        {
            return this.quote;
        }
    }
}
