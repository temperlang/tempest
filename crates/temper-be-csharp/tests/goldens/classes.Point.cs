namespace Classes;

internal class Point
{
    private readonly int x;
    private readonly int y;

    public Point(int x, int y)
    {
        this.x = x;
        this.y = y;
    }

    public int X
    {
        get
        {
            return this.x;
        }
    }

    public int Y
    {
        get
        {
            return this.y;
        }
    }

    public bool Eq(Point other)
    {
        if (this.x == other.x)
        {
            return this.y == other.y;
        }
        else
        {
            return false;
        }
    }
}
