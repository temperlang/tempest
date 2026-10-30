namespace Types.Exporter;

public class ExportedClass
{
    private readonly int foo;

    public ExportedClass(int foo)
    {
        this.foo = foo;
    }

    public int Foo
    {
        get
        {
            return this.foo;
        }
    }
}
