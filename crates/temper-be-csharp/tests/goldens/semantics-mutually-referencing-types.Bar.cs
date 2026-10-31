namespace MutuallyReferencingTypes;

internal class Bar
{
    private readonly Foo? x;

    public Bar(Foo? x)
    {
        this.x = x;
    }

    public Foo? X
    {
        get
        {
            return this.x;
        }
    }
}
