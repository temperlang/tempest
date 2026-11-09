namespace PureVirtual;

internal class Box
{
    private readonly string label;

    public Box(string label)
    {
        this.label = label;
    }

    public string Label
    {
        get
        {
            return this.label;
        }
    }
}
