namespace PrivateMethod;

internal class Label
{
    private readonly int end;

    public Label(int end)
    {
        this.end = end;
    }

    public string Text
    {
        get
        {
            return "label";
        }
    }

    public int Close(ISink sink)
    {
        return sink.End() + this.end;
    }

    public string Noted(string more)
    {
        return new Note(this.Text).Text(more).Body;
    }
}
