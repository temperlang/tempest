namespace Setters;

internal class Foo
{
    private string backing;

    public Foo(string backing)
    {
        this.backing = backing;
    }

    public string Bar
    {
        get
        {
            return this.backing;
        }
        set
        {
            this.backing = value;
        }
    }
}
