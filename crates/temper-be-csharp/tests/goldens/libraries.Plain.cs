namespace Libraries;

public class Plain<T> : IEffect<T>
{
    private readonly string text;

    public Plain(string text)
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
}
