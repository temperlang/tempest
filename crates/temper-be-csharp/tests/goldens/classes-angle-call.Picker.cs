using System.Collections.Generic;

namespace AngleCall;

internal class Picker
{
    public Picker()
    {
    }

    public T Pick<T>(IReadOnlyList<T> things)
    {
        return AngleCallGlobal.LastOf(things);
    }
}
