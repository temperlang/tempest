namespace MutuallyReferencingTypes;

internal class Foo
{
    private readonly Bar? x;

    public Foo(Bar? x)
    {
        this.x = x;
    }

    public Bar? X
    {
        get
        {
            return this.x;
        }
    }
}
