namespace Libraries;

public class Word
{
    private readonly string text;

    public Word(string text)
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
