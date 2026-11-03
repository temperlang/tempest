namespace ObjectLiterals.ExtraClass;

public class Rectangle
{
    private readonly int width;
    private readonly int height;

    public Rectangle(int width, int height)
    {
        this.width = width;
        this.height = height;
    }

    public int Width
    {
        get
        {
            return this.width;
        }
    }

    public int Height
    {
        get
        {
            return this.height;
        }
    }
}
