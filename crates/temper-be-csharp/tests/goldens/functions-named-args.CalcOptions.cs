namespace NamedArgs;

internal class CalcOptions
{
    private readonly int scale;
    private readonly int origin;

    public CalcOptions(int? scale = null, int? origin = null)
    {
        int scale2 = scale ?? 1;
        int origin2 = origin ?? 0;
        this.scale = scale2;
        this.origin = origin2;
    }

    public int Scale
    {
        get
        {
            return this.scale;
        }
    }

    public int Origin
    {
        get
        {
            return this.origin;
        }
    }
}
