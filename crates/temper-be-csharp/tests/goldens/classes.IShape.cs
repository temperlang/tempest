using System.Globalization;

namespace Classes;

public interface IShape
{
    int Area { get; }

    string Describe()
    {
        return DescribeDefault(this);
    }

    protected static string DescribeDefault(IShape @this)
    {
        return "shape of area " + @this.Area.ToString(CultureInfo.InvariantCulture);
    }
}
