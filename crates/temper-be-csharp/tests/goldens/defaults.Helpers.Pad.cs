namespace Defaults.Helpers;

public class Pad
{
    private readonly int left;
    private readonly int right;

    public Pad(int? left = null, int? right = null)
    {
        int left2 = left ?? 1;
        int right2 = right ?? 2;
        this.left = left2;
        this.right = right2;
    }

    public int Left
    {
        get
        {
            return this.left;
        }
    }

    public int Right
    {
        get
        {
            return this.right;
        }
    }

    public string Apply(string text, string? fill = null)
    {
        string fill2 = fill ?? ".";
        string @out = text;
        {
            int i = 0;
            while (i < this.left)
            {
                @out = fill2 + @out;
                i = i + 1;
            }
        }
        {
            int i = 0;
            while (i < this.right)
            {
                @out = @out + fill2;
                i = i + 1;
            }
        }
        return @out;
    }
}
