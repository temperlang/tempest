namespace PrivateMethod;

internal class C
{
    public C()
    {
    }

    private int Helper(int x)
    {
        return x + 1;
    }

    public int Method1(int x)
    {
        return this.Helper(x * 2);
    }

    public int Method2(int x)
    {
        return this.Helper(x * 3);
    }
}
