namespace Renamed;

public static class RenamedGlobal
{
    public static readonly int Native;

    public static int Lambda(int n)
    {
        return n + 1;
    }

    static RenamedGlobal()
    {
        Native = 2;
    }
}
