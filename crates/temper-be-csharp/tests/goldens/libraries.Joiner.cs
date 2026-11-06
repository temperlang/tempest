using System.Collections.Generic;
using System.Globalization;

namespace Libraries;

public class Joiner
{
    private string @out;

    public Joiner()
    {
        this.@out = "";
    }

    public string Out
    {
        get
        {
            return this.@out;
        }
        set
        {
            this.@out = value;
        }
    }

    public void AddWord(Word w)
    {
        this.@out = this.@out + "w" + w.Text;
    }

    public void AddFlags(IReadOnlyList<bool> x)
    {
        this.@out = this.@out + "f" + x.Count.ToString(CultureInfo.InvariantCulture);
    }

    public void AddTexts(IReadOnlyList<string> x)
    {
        this.@out = this.@out + "t" + x.Count.ToString(CultureInfo.InvariantCulture);
    }

    public void AddMaybes(IReadOnlyList<string?> x)
    {
        this.@out = this.@out + "m" + x.Count.ToString(CultureInfo.InvariantCulture);
    }
}
