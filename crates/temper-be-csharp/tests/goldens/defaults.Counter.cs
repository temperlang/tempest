namespace Defaults;

internal class Counter
{
    private int count;

    public Counter(int? start = null)
    {
        int start2 = start ?? 10;
        this.count = start2;
    }

    public int Count
    {
        get
        {
            return this.count;
        }
        set
        {
            this.count = value;
        }
    }

    public int Add(int? by = null)
    {
        int by2 = by ?? 1;
        this.count = this.count + by2;
        return this.count;
    }
}
