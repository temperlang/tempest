namespace Asserts;

public class Vec2
{
    private readonly double x;
    private readonly double y;

    public Vec2(double x, double y)
    {
        this.x = x;
        this.y = y;
    }

    public double X
    {
        get
        {
            return this.x;
        }
    }

    public double Y
    {
        get
        {
            return this.y;
        }
    }

    public override string ToString()
    {
        return "(" + AssertsGlobal.FormatFloat64(this.x) + ", " + AssertsGlobal.FormatFloat64(this.y) + ") size squared " + AssertsGlobal.FormatFloat64(AssertsGlobal.NormSquared(this));
    }
}
