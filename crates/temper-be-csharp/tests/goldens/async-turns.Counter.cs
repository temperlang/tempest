using System.Globalization;

namespace AsyncTurns;

internal class Counter
{
    private int n;

    public Counter()
    {
        this.n = 0;
    }

    public int N
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

    public void Bump(string tag, int times)
    {
        AsyncTurnsGlobal.console.Log(tag + " start");
        {
            int i = 0;
            while (i < times)
            {
                int before = this.n;
                this.n = before + 1;
                i = i + 1;
            }
        }
        AsyncTurnsGlobal.console.Log(tag + " done n=" + this.n.ToString(CultureInfo.InvariantCulture));
    }
}
