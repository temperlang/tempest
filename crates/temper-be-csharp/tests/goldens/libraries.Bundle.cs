using System.Collections.Generic;

namespace Libraries;

public class Bundle<T>
{
    private readonly IReadOnlyList<IEffect<T>> effects;
    private readonly T first;

    public Bundle(IReadOnlyList<IEffect<T>> effects, T first)
    {
        this.effects = effects;
        this.first = first;
    }

    public IReadOnlyList<IEffect<T>> Effects
    {
        get
        {
            return this.effects;
        }
    }

    public T First
    {
        get
        {
            return this.first;
        }
    }
}
