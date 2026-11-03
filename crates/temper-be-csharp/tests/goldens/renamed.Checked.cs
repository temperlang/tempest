namespace Renamed;

public class Checked
{
    private readonly int end;

    public Checked(int end)
    {
        this.end = end;
    }

    public int End
    {
        get
        {
            return this.end;
        }
    }

    public int Delete()
    {
        return this.end * 2;
    }

    public int Local
    {
        get
        {
            return this.end + RenamedGlobal.Native;
        }
    }
}
