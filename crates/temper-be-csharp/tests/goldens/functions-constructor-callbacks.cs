using System;

namespace ConstructorCallbacks;

public static class ConstructorCallbacksGlobal
{
    public static Hi<T> MakeHi<T>(Func<T, T> ha)
    {
        return new Hi<T>(ha);
    }

    public static Hi<int> NewIntHi(int j)
    {
        return new Hi<int>((int i) => i + j);
    }

    public static Hi<int> MakeIntHi(int j)
    {
        return MakeHi((int i) => i + j);
    }
}
