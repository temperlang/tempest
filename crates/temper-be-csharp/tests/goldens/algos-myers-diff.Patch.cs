using System.Collections.Generic;

namespace MyersDiff;

public class Patch<T>
{
    private readonly IReadOnlyList<Change<T>> changes;

    public Patch(IReadOnlyList<Change<T>> changes)
    {
        this.changes = changes;
    }

    public IReadOnlyList<Change<T>> Changes
    {
        get
        {
            return this.changes;
        }
    }
}
