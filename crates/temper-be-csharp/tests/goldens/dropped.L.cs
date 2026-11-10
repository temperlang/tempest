namespace Dropped;

internal class L : ILexer
{
    private int n;

    public L()
    {
        this.n = 0;
    }

    public int N
    {
        get
        {
            return this.n;
        }
        set
        {
            this.n = value;
        }
    }

    public string? Scan(string text)
    {
        this.n = this.n + 1;
        if (text == "x")
        {
            return null;
        }
        else
        {
            return text;
        }
    }

    public int Count
    {
        get
        {
            this.n = this.n + 10;
            return this.n;
        }
    }
}
