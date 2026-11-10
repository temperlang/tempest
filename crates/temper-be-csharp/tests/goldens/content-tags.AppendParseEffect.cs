namespace ContentTags;

public class AppendParseEffect : IEffect
{
    private readonly string text;

    public AppendParseEffect(string text)
    {
        this.text = text;
    }

    public string Text
    {
        get
        {
            return this.text;
        }
    }

    public void EnactOn(Buffer c)
    {
        c.Append(this.text);
    }
}
