namespace PureVirtual;

internal class BoxWrapper : IWrapper<Box>
{
    public BoxWrapper()
    {
    }

    public Box Wrap(Box item)
    {
        item = new Box("[" + item.Label + "]");
        return item;
    }
}
