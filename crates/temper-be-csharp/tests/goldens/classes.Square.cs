using System.Globalization;

namespace Classes;

public class Square : IShape
{
    private readonly int side;

    public Square(int side)
    {
        this.side = side;
    }

    public int Area
    {
        get
        {
            return this.side * this.side;
        }
    }

    public string Describe()
    {
        return "square of side " + this.side.ToString(CultureInfo.InvariantCulture);
    }
}
