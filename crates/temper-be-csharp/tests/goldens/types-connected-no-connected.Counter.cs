namespace Work;

public class Counter
{
    private int count;

    public Counter()
    {
        this.count = 0;
    }

    public void Add(int i)
    {
        this.count = this.count + i;
    }

    public int Total()
    {
        return this.count + 1000;
    }
}
