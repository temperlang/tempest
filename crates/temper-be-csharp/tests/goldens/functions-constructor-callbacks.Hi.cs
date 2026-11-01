using System;

namespace ConstructorCallbacks;

public class Hi<T>
{
    private readonly Func<T, T> ha;

    public Hi(Func<T, T> ha)
    {
        this.ha = ha;
    }

    public Func<T, T> Ha
    {
        get
        {
            return this.ha;
        }
    }
}
