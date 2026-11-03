using System.Collections.Generic;

namespace MyersDiff;

public class Change<T>
{
    private readonly string type;
    private readonly int leftIndex;
    private readonly int rightIndex;
    private readonly IReadOnlyList<T> items;

    public Change(string type, int leftIndex, int rightIndex, IReadOnlyList<T> items)
    {
        this.type = type;
        this.leftIndex = leftIndex;
        this.rightIndex = rightIndex;
        this.items = items;
    }

    public string Type
    {
        get
        {
            return this.type;
        }
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

    public IReadOnlyList<T> Items
    {
        get
        {
            return this.items;
        }
    }
}
