namespace AngleCall;

internal class Hi
{
    private readonly Blah<string> thing;

    public Hi(Blah<string> thing)
    {
        this.thing = thing;
    }

    public Blah<string> Thing
    {
        get
        {
            return this.thing;
        }
    }
}
