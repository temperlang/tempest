using System.Globalization;

namespace Classes;

public class Rect : IShape
{
    private readonly int w;
    private readonly int h;

    public Rect(int w, int h)
    {
        this.w = w;
        this.h = h;
    }

    public int W
    {
        get
        {
            return this.w;
        }
    }

    public int H
    {
        get
        {
            return this.h;
        }
    }

    public static Rect Unit
    {
        get
        {
            return ClassesGlobal.rectUnit;
        }
    }

    public int Area
    {
        get
        {
            return this.w * this.h;
        }
    }

    public override string ToString()
    {
        return string.Create(CultureInfo.InvariantCulture, $"Rect({this.w}, {this.h})");
    }

    public Rect Scaled(int k)
    {
        return new Rect(this.w * k, this.h * k);
    }

    public string Describe()
    {
        return IShape.DescribeDefault(this);
    }
}
