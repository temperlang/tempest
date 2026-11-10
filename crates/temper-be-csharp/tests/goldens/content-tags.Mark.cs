namespace ContentTags;

public class Mark
{
    private readonly string label;

    public Mark(string label)
    {
        this.label = label;
    }

    public string Label
    {
        get
        {
            return this.label;
        }
    }
}
