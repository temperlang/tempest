namespace Work;

public class Hider
{
    private readonly int i;
    private readonly Hidden hidden;

    public Hider(int i)
    {
        this.i = i;
        this.hidden = new Hidden(this.i);
    }

    public int Plus(int j)
    {
        return WorkGlobal.Sum(this.hidden.I, j);
    }

    public int Times(int j)
    {
        return WorkGlobal.Prod(this.hidden, j);
    }
}
