using System.Globalization;

namespace ExtraClass;

public class Circle : IStringable
{
    private readonly int radius;

    public Circle(int radius)
    {
        this.radius = radius;
    }

    public int Radius
    {
        get
        {
            return this.radius;
        }
    }

    public override string ToString()
    {
        return "Circle of radius " + this.radius.ToString(CultureInfo.InvariantCulture);
    }
}
