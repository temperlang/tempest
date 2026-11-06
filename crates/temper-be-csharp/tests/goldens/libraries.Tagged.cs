namespace Libraries;

public class Tagged
{
    private readonly string? label;

    public Tagged(string? label)
    {
        this.label = label;
    }

    public string? Label
    {
        get
        {
            return this.label;
        }
    }
}
