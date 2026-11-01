using System;

namespace AsValues;

internal class DelayedFunction
{
    private readonly Action<string> f;

    public DelayedFunction(Action<string> f)
    {
        this.f = f;
    }

    public void CallIt()
    {
        this.f("from a method");
    }
}
