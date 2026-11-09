namespace PropertyMembers;

internal class LabeledSub : ILabeled<string>
{
    private readonly C thing;

    public LabeledSub(C thing)
    {
        this.thing = thing;
    }

    public C Thing
    {
        get
        {
            return this.thing;
        }
    }

    public string Label(string l)
    {
        return l + ": " + this.thing.X;
    }
}
