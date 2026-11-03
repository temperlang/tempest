namespace MyersDiff;

internal class DiffPath
{
    private readonly int leftIndex;
    private readonly int rightIndex;
    private readonly string changeType;
    private readonly DiffPath? previous;

    public DiffPath(int leftIndex, int rightIndex, string changeType, DiffPath? previous)
    {
        this.leftIndex = leftIndex;
        this.rightIndex = rightIndex;
        this.changeType = changeType;
        this.previous = previous;
    }

    public int LeftIndex
    {
        get
        {
            return this.leftIndex;
        }
    }

    public int RightIndex
    {
        get
        {
            return this.rightIndex;
        }
    }

    public string ChangeType
    {
        get
        {
            return this.changeType;
        }
    }

    public DiffPath? Previous
    {
        get
        {
            return this.previous;
        }
    }
}
