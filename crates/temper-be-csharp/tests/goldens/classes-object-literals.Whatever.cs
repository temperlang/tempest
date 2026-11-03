using System.Globalization;

namespace ObjectLiterals;

internal class Whatever
{
    private readonly string blah;

    public Whatever(int blech)
    {
        this.blah = blech.ToString(CultureInfo.InvariantCulture);
    }

    public string Blah
    {
        get
        {
            return this.blah;
        }
    }
}
