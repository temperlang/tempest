using System.Globalization;

namespace ObjectLiterals;

internal class Moreover
{
    private readonly int blech;
    private readonly int bling;

    public Moreover(int? blech = null, int? bling = null)
    {
        int blech2 = blech ?? 0;
        int bling2 = bling ?? 0;
        this.blech = blech2;
        this.bling = bling2;
    }

    public int Blech
    {
        get
        {
            return this.blech;
        }
    }

    public int Bling
    {
        get
        {
            return this.bling;
        }
    }

    public override string ToString()
    {
        return string.Create(CultureInfo.InvariantCulture, $"{this.blech} {this.bling}");
    }
}
