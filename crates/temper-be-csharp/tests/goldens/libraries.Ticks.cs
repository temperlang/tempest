using System.Collections.Generic;
using System.Globalization;

namespace Libraries;

public class Ticks : ICounting<string>
{
    private int n;

    public Ticks()
    {
        this.n = 0;
    }

    public int Count
    {
        get
        {
            return this.n;
        }
        set
        {
            this.n = value;
        }
    }

    public string Made()
    {
        return "tick" + this.n.ToString(CultureInfo.InvariantCulture);
    }

    public IReadOnlyList<string> Step()
    {
        return ICounting<string>.StepDefault(this);
    }
}
