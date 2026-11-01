using System.Globalization;

namespace Sorting;

internal class Entry
{
    private readonly int id;
    private readonly int group;

    public Entry(int id, int group)
    {
        this.id = id;
        this.group = group;
    }

    public int Id
    {
        get
        {
            return this.id;
        }
    }

    public int Group
    {
        get
        {
            return this.group;
        }
    }

    public override string ToString()
    {
        return string.Create(CultureInfo.InvariantCulture, $"{{ id: {this.id}, group: {this.group} }}");
    }
}
